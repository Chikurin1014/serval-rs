//! Plugins: Rune files whose public async functions become tasks, compiled as
//! they are imported and run as the tasks are turned on (`run`).

mod run;
mod serval;
mod task;

pub use run::{CompiledPlugin, INSTRUCTIONS_PER_STEP, compile_plugin, run_task};
pub use serval::{Host, LogEntry, set_host};
pub use task::{
    ImportError, Imported, NewLines, PrintedLine, Task, TaskContext, TaskOutcome, TaskProvider,
};

use rune::{
    SourceId,
    ast::{self, Spanned},
    parse::parse_all,
};

/// A `pub async fn` of a plugin.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginFunction {
    pub name: String,
    /// As the file has it, from its doc comments and attributes to its closing
    /// brace.
    pub definition: String,
}

/// The top-level `pub async fn`s of `source`, in order; else why it is not Rune.
pub fn public_async_functions(source: &str) -> Result<Vec<PluginFunction>, String> {
    let file = parse_all::<ast::File>(source, SourceId::empty(), true).map_err(|error| {
        let line = source[..error.span().range().start].matches('\n').count() + 1;
        format!("Line {line}: {error}")
    })?;
    Ok(file
        .items
        .iter()
        .filter_map(|(item, _)| match item {
            ast::Item::Fn(function)
                if matches!(function.visibility, ast::Visibility::Public(_))
                    && function.async_token.is_some() =>
            {
                Some(PluginFunction {
                    name: source[function.name.span().range()].to_string(),
                    definition: source[function.span().range()].to_string(),
                })
            }
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{PluginFunction, public_async_functions};

    fn names(source: &str) -> Vec<String> {
        public_async_functions(source)
            .unwrap()
            .into_iter()
            .map(|function| function.name)
            .collect()
    }

    #[test]
    fn public_async_functions_are_found_with_their_definitions() {
        let source = "use std::string;\n\n\
            /// Blinks the LED.\n\
            pub async fn blink(led) {\n    let text = `on {led}`;\n}\n\n\
            async fn helper() {}\n\
            pub fn sync() {}\n\
            pub async fn poll() { #{ ok: true } }\n";
        assert_eq!(
            public_async_functions(source).unwrap(),
            [
                PluginFunction {
                    name: "blink".into(),
                    definition: "/// Blinks the LED.\n\
                        pub async fn blink(led) {\n    let text = `on {led}`;\n}"
                        .into(),
                },
                PluginFunction {
                    name: "poll".into(),
                    definition: "pub async fn poll() { #{ ok: true } }".into(),
                },
            ]
        );
    }

    #[test]
    fn other_functions_are_not() {
        let source = "pub async fn outer() {\n    async fn inner() {}\n}\n\
            async fn private() {}\n\
            pub(crate) async fn crate_only() {}\n\
            mod tools { pub async fn nested() {} }\n\
            struct Led;\n\
            impl Led { pub async fn on(self) {} }\n";
        assert_eq!(names(source), ["outer"]);
    }

    #[test]
    fn braces_in_strings_comments_and_labels_are_passed_over() {
        let source = "// pub async fn commented() {}\n\
            /* pub async fn blocked() { */\n\
            pub async fn real() {\n    let s = \"}\";\n    let c = '}';\n    \
            'outer: loop { break 'outer; }\n}\n\
            pub async fn after() {}\n";
        assert_eq!(names(source), ["real", "after"]);
    }

    #[test]
    fn a_file_not_of_rune_is_told_where() {
        let error =
            public_async_functions("pub async fn ok() {}\npub async fn broken( {\n").unwrap_err();
        assert!(error.starts_with("Line 2: "), "{error}");
        assert_eq!(names(""), Vec::<String>::new());
    }
}
