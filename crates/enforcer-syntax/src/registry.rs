//! Canonical parser identity registry for UL06.
//!
//! This P0 slice owns identity and structural disposition only. Literal,
//! scan-family, native-tool, CLI, and MCP projections are migrated in later
//! serial packets; this registry must not imply semantic rule support.

use crate::parsers::Language;
use enforcer_domain::language_types::{LanguageId, StructuralLanguageSupport};

/// One canonical parser identity and its structural parse disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageRecord {
    /// Stable one-based canonical identity.
    pub id: LanguageId,
    /// Existing parser enum variant preserved by this migration.
    pub parser: Language,
    /// Whether parser dispatch returns a structural result.
    pub structural: StructuralLanguageSupport,
}

/// The reviewed parser identity projection in stable source order.
pub static LANGUAGE_RECORDS: &[LanguageRecord] = &[
    LanguageRecord {
        id: LanguageId::from_registry_index(1),
        parser: Language::Rust,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(2),
        parser: Language::TypeScript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(3),
        parser: Language::JavaScript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(4),
        parser: Language::Python,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(5),
        parser: Language::Go,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(6),
        parser: Language::Java,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(7),
        parser: Language::C,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(8),
        parser: Language::Cpp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(9),
        parser: Language::CSharp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(10),
        parser: Language::Php,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(11),
        parser: Language::Kotlin,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(12),
        parser: Language::Swift,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(13),
        parser: Language::Tsx,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(14),
        parser: Language::Solidity,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(15),
        parser: Language::Gdscript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(16),
        parser: Language::Dart,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(17),
        parser: Language::Scala,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(18),
        parser: Language::Groovy,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(19),
        parser: Language::Ruby,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(20),
        parser: Language::Zig,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(21),
        parser: Language::ObjectiveC,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(22),
        parser: Language::Bash,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(23),
        parser: Language::Lua,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(24),
        parser: Language::Elixir,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(25),
        parser: Language::Haskell,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(26),
        parser: Language::OCaml,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(27),
        parser: Language::Erlang,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(28),
        parser: Language::Cuda,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(29),
        parser: Language::D,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(30),
        parser: Language::PowerShell,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(31),
        parser: Language::Fsharp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(32),
        parser: Language::Gleam,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(33),
        parser: Language::Glsl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(34),
        parser: Language::Ada,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(35),
        parser: Language::Apex,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(36),
        parser: Language::Crystal,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(37),
        parser: Language::R,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(38),
        parser: Language::Perl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(39),
        parser: Language::Clojure,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(40),
        parser: Language::ConfigToml,
        structural: StructuralLanguageSupport::NoParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(41),
        parser: Language::ConfigJson,
        structural: StructuralLanguageSupport::NoParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(42),
        parser: Language::ConfigYaml,
        structural: StructuralLanguageSupport::NoParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(43),
        parser: Language::Julia,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(44),
        parser: Language::Odin,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(45),
        parser: Language::Pascal,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(46),
        parser: Language::Qml,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(47),
        parser: Language::Rescript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(48),
        parser: Language::Squirrel,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(49),
        parser: Language::Sway,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(50),
        parser: Language::Starlark,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(51),
        parser: Language::Templ,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(52),
        parser: Language::Typst,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(53),
        parser: Language::Wgsl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(54),
        parser: Language::Wolfram,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(55),
        parser: Language::Slang,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(56),
        parser: Language::Scss,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(57),
        parser: Language::Cmake,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(58),
        parser: Language::Makefile,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(59),
        parser: Language::Fortran,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(60),
        parser: Language::Vimscript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(61),
        parser: Language::Puppet,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(62),
        parser: Language::Elm,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(63),
        parser: Language::Bicep,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(64),
        parser: Language::Bitbake,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(65),
        parser: Language::Cairo,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(66),
        parser: Language::Cfscript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(67),
        parser: Language::Func,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(68),
        parser: Language::Move,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(69),
        parser: Language::Nickel,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(70),
        parser: Language::Jsonnet,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(71),
        parser: Language::Just,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(72),
        parser: Language::Hlsl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(73),
        parser: Language::Ispc,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(74),
        parser: Language::Purescript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(75),
        parser: Language::Magma,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(76),
        parser: Language::Hare,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(77),
        parser: Language::Pony,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(78),
        parser: Language::Nasm,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(79),
        parser: Language::Cobol,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(80),
        parser: Language::Commonlisp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(81),
        parser: Language::Lean,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(82),
        parser: Language::Tlaplus,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(83),
        parser: Language::Verilog,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(84),
        parser: Language::Vhdl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(85),
        parser: Language::Systemverilog,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(86),
        parser: Language::Capnp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(87),
        parser: Language::EmacsLisp,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(88),
        parser: Language::Agda,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(89),
        parser: Language::Form,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(90),
        parser: Language::Awk,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(91),
        parser: Language::Fish,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(92),
        parser: Language::Zsh,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(93),
        parser: Language::Tcl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(94),
        parser: Language::Scheme,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(95),
        parser: Language::Racket,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(96),
        parser: Language::Smithy,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(97),
        parser: Language::Pine,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(98),
        parser: Language::Matlab,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(99),
        parser: Language::Luau,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(100),
        parser: Language::Teal,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(101),
        parser: Language::Fennel,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(102),
        parser: Language::Meson,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(103),
        parser: Language::Kconfig,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(104),
        parser: Language::Hcl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(105),
        parser: Language::Nix,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(106),
        parser: Language::Sql,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(107),
        parser: Language::Protobuf,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(108),
        parser: Language::Prisma,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(109),
        parser: Language::Pkl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(110),
        parser: Language::Thrift,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(111),
        parser: Language::Wit,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(112),
        parser: Language::LlvmIr,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(113),
        parser: Language::TableGen,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(114),
        parser: Language::Cfml,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(115),
        parser: Language::Gotemplate,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(116),
        parser: Language::Devicetree,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(117),
        parser: Language::Smali,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(118),
        parser: Language::Json5,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(119),
        parser: Language::Kdl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(120),
        parser: Language::LinkerScript,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(121),
        parser: Language::Liquid,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(122),
        parser: Language::Markdown,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(123),
        parser: Language::Mermaid,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(124),
        parser: Language::Po,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(125),
        parser: Language::Properties,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(126),
        parser: Language::Regex,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(127),
        parser: Language::Assembly,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(128),
        parser: Language::Astro,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(129),
        parser: Language::Beancount,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(130),
        parser: Language::Bibtex,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(131),
        parser: Language::Blade,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(132),
        parser: Language::Css,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(133),
        parser: Language::Csv,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(134),
        parser: Language::Diff,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(135),
        parser: Language::Dockerfile,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(136),
        parser: Language::Dotenv,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(137),
        parser: Language::Gitattributes,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(138),
        parser: Language::Gitignore,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(139),
        parser: Language::Gn,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(140),
        parser: Language::GoMod,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(141),
        parser: Language::Graphql,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(142),
        parser: Language::Html,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(143),
        parser: Language::Hyprlang,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(144),
        parser: Language::Ini,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(145),
        parser: Language::Janet,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(146),
        parser: Language::Jinja2,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(147),
        parser: Language::Jsdoc,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(148),
        parser: Language::Json,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(149),
        parser: Language::TextOnly,
        structural: StructuralLanguageSupport::NoParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(150),
        parser: Language::Requirements,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(151),
        parser: Language::Ron,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(152),
        parser: Language::Rst,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(153),
        parser: Language::Soql,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(154),
        parser: Language::Sosl,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(155),
        parser: Language::Sshconfig,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(156),
        parser: Language::Svelte,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(157),
        parser: Language::Toml,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(158),
        parser: Language::Vue,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(159),
        parser: Language::Xml,
        structural: StructuralLanguageSupport::ParseFile,
    },
    LanguageRecord {
        id: LanguageId::from_registry_index(160),
        parser: Language::Yaml,
        structural: StructuralLanguageSupport::ParseFile,
    },
];

/// Return all canonical parser identities in stable registry order.
pub fn language_registry() -> &'static [LanguageRecord] {
    LANGUAGE_RECORDS
}

/// Find the canonical record for one existing parser variant.
pub fn record_for_parser(language: Language) -> Option<&'static LanguageRecord> {
    language_registry()
        .iter()
        .find(|record| record.parser == language)
}
