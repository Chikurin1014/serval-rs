# Changelog

## [0.4.0](https://github.com/Chikurin1014/serval-rs/compare/v0.3.1...v0.4.0) (2026-10-09)


### Features

* ✨ Add DTR/RTS operation ([90cfc36](https://github.com/Chikurin1014/serval-rs/commit/90cfc36febd44db5914991c84299c3a245d58873))
* ✨ Add HEX format mode ([ff534e1](https://github.com/Chikurin1014/serval-rs/commit/ff534e195a9b4af6679a5e891621c3bd22bf39b6))
* ✨ Add logger ([74cb274](https://github.com/Chikurin1014/serval-rs/commit/74cb27402120866459da09c76c9001534795116d))
* ✨ Add port info to text log ([a4bb72a](https://github.com/Chikurin1014/serval-rs/commit/a4bb72ae2dd94a7dc6c1890f87a94107887074d4))
* ✨ Add send file button ([5f2716d](https://github.com/Chikurin1014/serval-rs/commit/5f2716d0d56531b70d9dc3041816cc972776825f))
* ✨ Add send file button ([e4d43db](https://github.com/Chikurin1014/serval-rs/commit/e4d43db27d6778115bdd4674d322878b5e086748))
* ✨ Add timestamp to console ([ca352a7](https://github.com/Chikurin1014/serval-rs/commit/ca352a7cc11efe1b8bfc46602b2716b3d9e5809c))
* ✨ Implement `LogSink` for web platform ([4be7339](https://github.com/Chikurin1014/serval-rs/commit/4be73390aff9067e0390e696231523758f522752))
* 💄 Add a button to start / end logging ([aa0d63a](https://github.com/Chikurin1014/serval-rs/commit/aa0d63a79b74d414b342f03921fc75f917ddaa13))
* 💄 Add alert notification ([b2f5f78](https://github.com/Chikurin1014/serval-rs/commit/b2f5f78bbc19957754c18ee7d6ec9c5fbf129ad8))


### Bug Fixes

* 🐛 Fix performance drop due to timestamp line following console line ([2f2a2f5](https://github.com/Chikurin1014/serval-rs/commit/2f2a2f5bafcb37bc4fd7402be9d0282eb316fc2b))

## [0.3.1](https://github.com/Chikurin1014/serval-rs/compare/v0.3.0...v0.3.1) (2026-10-08)


### Bug Fixes

* 🐛 Fix memory leak on Data tab ([95beca3](https://github.com/Chikurin1014/serval-rs/commit/95beca3006efe37c0e07e77c1a515aef4ee0cdc6))
* 🐛 Fix memory leak on Data tab ([70aea7f](https://github.com/Chikurin1014/serval-rs/commit/70aea7fae520cb6643e05bf1f5e8617eed56752b))

## [0.3.0](https://github.com/Chikurin1014/serval-rs/compare/v0.2.1...v0.3.0) (2026-10-08)


### Features

* ✨ Add send-data buffer visualization ([b136bbd](https://github.com/Chikurin1014/serval-rs/commit/b136bbd81e4da9010f84dccf7c73badd6773af91))
* ✨ Implement some feature of Tera Term ([ebc6874](https://github.com/Chikurin1014/serval-rs/commit/ebc6874dc26fc1a97acc2ed614f8311a4a61783d))
* ✨ Introduce `Xterm.js` ([020fb39](https://github.com/Chikurin1014/serval-rs/commit/020fb394e7b341c3f5815d1d28d288a9af48be7a))
* ✨ Support ANSI escape sequences ([330fe0d](https://github.com/Chikurin1014/serval-rs/commit/330fe0d2774d8df54978a1fbde2e2d7d80263b18))
* 💄 Replace favicon with self-drawn rough sketch ([fc3e9cb](https://github.com/Chikurin1014/serval-rs/commit/fc3e9cbe8f5b4813280121068077829902554ae7))
* 💄 Replace favicon with self-drawn rough sketch ([a0e79f6](https://github.com/Chikurin1014/serval-rs/commit/a0e79f65c0a8aec1834d08ba7c180dcf37202c5f))


### Bug Fixes

* 🩹 Fix positoin of tooltip ([eee9f8d](https://github.com/Chikurin1014/serval-rs/commit/eee9f8ded326809eaeeb8d34ecbc2541a65d8603))
* 🩹 Modify default regex ([0359303](https://github.com/Chikurin1014/serval-rs/commit/03593034d925bf9ac9327bd49c359315f0d0c049))

## [0.2.1](https://github.com/Chikurin1014/serval-rs/compare/v0.2.0...v0.2.1) (2026-10-08)


### Bug Fixes

* 🩹 small fix ([d19406d](https://github.com/Chikurin1014/serval-rs/commit/d19406d12aad14b1e99cc1479d233aec92b17e14))
* 🩹 Unformat third party libraries ([9963f93](https://github.com/Chikurin1014/serval-rs/commit/9963f93050b4f972292c0c3cb4aa1c78aed8989d))


### Performance Improvements

* ⚡️ Reduce frequency of regex compilation ([9ee448a](https://github.com/Chikurin1014/serval-rs/commit/9ee448a12c9e123a387435797f8f4f19ba0285c7))
* ⚡️ Reduce rendering frequency ([8c3dcba](https://github.com/Chikurin1014/serval-rs/commit/8c3dcba847f75b83130b1e844048327403f93338))

## [0.2.0](https://github.com/Chikurin1014/serval-rs/compare/v0.1.0...v0.2.0) (2026-10-07)


### Features

* ✨ `HEX`/`BIN`/`DEC` style to send data to serial ([056a65c](https://github.com/Chikurin1014/serval-rs/commit/056a65c5c43ac78fc0acd8e2e183d8f6940bf6d0))
* ✨ Add anonymous-data watcher to default maps ([16340cb](https://github.com/Chikurin1014/serval-rs/commit/16340cb30f8e76f6c1b3d246efa705bce8d3c739))
* ✨ add arithmetic operator Maps ([dba431a](https://github.com/Chikurin1014/serval-rs/commit/dba431ad3988b6f1eb63d35fbfe1ceed9ccc0a38))
* ✨ add culculus operations ([2974425](https://github.com/Chikurin1014/serval-rs/commit/29744258b1d5f89353af5da86c3c64285342d3aa))
* ✨ Add dialog to tell user the browser is not supported ([510844f](https://github.com/Chikurin1014/serval-rs/commit/510844f7168c5d55786e4b5eab8f67e939282a3b))
* ✨ Add favicon and modified tab title ([9337eb9](https://github.com/Chikurin1014/serval-rs/commit/9337eb9c2065a5e0df2d95050266b628ef805da5))
* ✨ Add frequently-used pattern aliases to regex ([14e8a8c](https://github.com/Chikurin1014/serval-rs/commit/14e8a8c75f929f5a87cb0ad6f0a7a7446eb3c5e6))
* ✨ add fundamental graph settings ([7478da7](https://github.com/Chikurin1014/serval-rs/commit/7478da7f132f58536c63e51e61940ab06a7659c8))
* ✨ add KaTeX ([1863c57](https://github.com/Chikurin1014/serval-rs/commit/1863c5752c56a59dabe67cccf424e6026b3ab108))
* ✨ add label filter ([af4218b](https://github.com/Chikurin1014/serval-rs/commit/af4218b4d4863fee761a5abd7f524a0c1b3ed6b5))
* ✨ add millisec property to TimeContext ([d666f12](https://github.com/Chikurin1014/serval-rs/commit/d666f121376cd0ac1fca311d20509891c46953db))
* ✨ add Regex presets ([f3dc108](https://github.com/Chikurin1014/serval-rs/commit/f3dc108b37c7eee249fd6e1f35eb135ee5aad8ec))
* ✨ add String Maps ([efda9dd](https://github.com/Chikurin1014/serval-rs/commit/efda9dd5247d616c1ab74cf02e5c8355a0a9f1ab))
* ✨ expandable datalist rows ([3fa0883](https://github.com/Chikurin1014/serval-rs/commit/3fa0883484a2dc481a74ddaf42a105035fb3eef8))
* ✨ export data as a CSV file ([8213338](https://github.com/Chikurin1014/serval-rs/commit/82133383261c1c7e9a32c967cbc6f5b4b98a0f0d))
* ✨ Introduce `fancy-regex` crate ([7e69182](https://github.com/Chikurin1014/serval-rs/commit/7e6918273ba98c90e9b1f41e8da2d4bfd4954e20))
* ✨ introduce cursor sync ([2b6b172](https://github.com/Chikurin1014/serval-rs/commit/2b6b172db1e683b01a96ea7cbb94c0378dac1838))
* ✨ new Map (Encode, Decode) ([d6577cc](https://github.com/Chikurin1014/serval-rs/commit/d6577cc33318d607644d90caa0870b442e1c31d8))
* 💄 add animation to collapsible elements ([a4517c5](https://github.com/Chikurin1014/serval-rs/commit/a4517c5d847ba57112d7df1d456fe423d71a53d1))
* 💄 add gradient fill for time_series chart ([bc3b8e3](https://github.com/Chikurin1014/serval-rs/commit/bc3b8e3caef362e4015e2fd4ca035bfd57b2b306))
* 💄 add gradient fill for time_series chart ([94c52e9](https://github.com/Chikurin1014/serval-rs/commit/94c52e9cec42ca0235ab35313656f64ef4d6d8c5))
* 💄 improve Map UI ([4beb197](https://github.com/Chikurin1014/serval-rs/commit/4beb1975fcbe686c2baf30b992265f35835b75a5))
* 💄 improve PortSelector UI ([4cc955e](https://github.com/Chikurin1014/serval-rs/commit/4cc955e5ad8c7e664732f8c2e5f25dad7e042531))
* 💄 improve UI ([77bf0c5](https://github.com/Chikurin1014/serval-rs/commit/77bf0c537538d27e6895981fc843577b77af8b9b))
* 💄 improve UI ([4a0ecb9](https://github.com/Chikurin1014/serval-rs/commit/4a0ecb91aadd637ace699fedd15109663b7d2028))
* 💄 replace plus icon with square-function ([4d2754e](https://github.com/Chikurin1014/serval-rs/commit/4d2754ef32bd37ac819a93ceda6e219bf78e40c8))
* 💄 Round toggle groups' corner ([9f1409c](https://github.com/Chikurin1014/serval-rs/commit/9f1409c869aab2922d98527e1b44ce2ff649e2a0))


### Bug Fixes

* 🐛 Specify path to `public` directory ([02ac508](https://github.com/Chikurin1014/serval-rs/commit/02ac5088dafea2d5733f80151d5789b62aab8e6a))
* 🩹 add a list of hidden data in the graph to `GraphContext` ([7a5cb03](https://github.com/Chikurin1014/serval-rs/commit/7a5cb038bd10aa56ee399deb710ec96031abfc4c))
* 🩹 Fix graph axis range ([a51e530](https://github.com/Chikurin1014/serval-rs/commit/a51e530db57e801fe8daa01c3c7d093ddc7f17ea))
* 🩹 Fix small issues with regex ([61971f3](https://github.com/Chikurin1014/serval-rs/commit/61971f3d3052e248a01d8729f69efc9499175d32))
* 🩹 Fix the expression of `{word}` ([f76db4e](https://github.com/Chikurin1014/serval-rs/commit/f76db4eb0bc48f3de91b5f5094b92ba769408ee3))
* 🩹 fix width of datalist column ([0de0a63](https://github.com/Chikurin1014/serval-rs/commit/0de0a63e1f175c1421b05823bd536fa694607cad))
