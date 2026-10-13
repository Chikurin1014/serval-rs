use dioxus::prelude::*;
use dioxus_icons::lucide;

use super::field::LabelField;
use super::form_of;
use super::map_list::{NUMBERS_LABELS_LIST_ID, VECTOR2D_LABELS_LIST_ID, VECTOR3D_LABELS_LIST_ID};
use crate::{
    data::{
        Calculus, CalculusMap, CalculusSettings, DataType, MapKind, MapRunner, Vector2d, Vector3d,
        VectorCalculus,
    },
    elements::Formula,
};

pub const DIFFERENTIATE: MapKind = calculus_kind("Differentiate", || {
    Box::new(CalculusMap::new(Calculus::Differentiate))
});
pub const INTEGRATE: MapKind = calculus_kind("Integrate", || {
    Box::new(CalculusMap::new(Calculus::Integrate))
});

const fn calculus_kind(name: &'static str, create: fn() -> Box<dyn MapRunner>) -> MapKind {
    MapKind {
        name,
        from: &[DataType::Number],
        to: DataType::Number,
        create,
        form: |settings| {
            form_of(
                settings,
                |settings: CalculusSettings| rsx! { CalculusForm { settings } },
            )
        },
        presets: &[],
    }
}

/// Its labels in the `datalist` `list`: Numbers, or else vectors.
#[component]
pub fn CalculusForm(
    settings: CalculusSettings,
    #[props(default = NUMBERS_LABELS_LIST_ID.to_string())] list: String,
    #[props(default)] vector: bool,
) -> Element {
    let CalculusSettings {
        calculus,
        from_label,
        to_label,
    } = settings;
    let latex = if vector {
        calculus.vector_latex()
    } else {
        calculus.latex()
    };

    rsx! {
        div {
            class: "map-form",
            Formula {
                class: "map-formula",
                latex: latex.to_string(),
                fallback: calculus.text().to_string(),
            }
            div {
                class: "map-row",
                LabelField {
                    name: "f(t)",
                    value: from_label,
                    list: list.clone(),
                    placeholder: "Input label",
                }
                lucide::MoveRight { size: 20 }
                LabelField {
                    value: to_label,
                    list,
                    placeholder: "Output label",
                }
            }
        }
    }
}

/// Differentiates or integrates each component of `$vector`s.
macro_rules! vector_calculus_kind {
    ($name:literal, $calculus:ident, $vector:ident, $list:expr) => {
        MapKind {
            name: $name,
            from: &[DataType::$vector],
            to: DataType::$vector,
            create: || -> Box<dyn MapRunner> {
                Box::new(VectorCalculus::<$vector>::new(Calculus::$calculus))
            },
            form: |settings| {
                form_of(settings, |settings: CalculusSettings| {
                    rsx! {
                        CalculusForm { settings, list: $list.to_string(), vector: true }
                    }
                })
            },
            presets: &[],
        }
    };
}

pub const DIFFERENTIATE_2D: MapKind = vector_calculus_kind!(
    "Differentiate",
    Differentiate,
    Vector2d,
    VECTOR2D_LABELS_LIST_ID
);
pub const DIFFERENTIATE_3D: MapKind = vector_calculus_kind!(
    "Differentiate",
    Differentiate,
    Vector3d,
    VECTOR3D_LABELS_LIST_ID
);
pub const INTEGRATE_2D: MapKind =
    vector_calculus_kind!("Integrate", Integrate, Vector2d, VECTOR2D_LABELS_LIST_ID);
pub const INTEGRATE_3D: MapKind =
    vector_calculus_kind!("Integrate", Integrate, Vector3d, VECTOR3D_LABELS_LIST_ID);
