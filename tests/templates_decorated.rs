use aheadlibex_rs::dll::ExportEntry;
use aheadlibex_rs::templates::{
    render_asm_x64, render_asm_x64_gas, render_asm_x86, render_c, render_c_x64, render_cmake_lists,
    render_def, render_vcxproj, OriginLoadMode, VsGuids, VsTemplateContext,
};

fn named_export(name: &str, ordinal: u16) -> ExportEntry {
    ExportEntry::named(name, ordinal, None)
}

fn ordinal_only_export(ordinal: u16) -> ExportEntry {
    ExportEntry::ordinal_only(ordinal, None)
}

fn dummy_ctx<'a>(exports: &'a [ExportEntry]) -> VsTemplateContext<'a> {
    let guids = VsGuids {
        solution: "{S}",
        project: "{P}",
        filter_source: "{FS}",
        filter_header: "{FH}",
        filter_resource: "{FR}",
    };
    VsTemplateContext {
        project_name: "Foo",
        dll_name: "Foo.dll",
        base_name: "Foo",
        origin_load_mode: OriginLoadMode::SystemDir,
        exports,
        guids,
    }
}

#[test]
fn decorated_names_are_preserved_in_exports() {
    let exports = vec![
        named_export("?Func@@YAXH@Z", 1),
        named_export("@Func@8", 2),
        named_export("??0Class@@QAE@XZ", 3),
        ordinal_only_export(345),
    ];

    let ctx = dummy_ctx(&exports);

    let c_x86 = render_c(&ctx);
    assert!(c_x86.contains("FARPROC pfnAheadLibEx__Func__YAXH_Z = nullptr;"));
    assert!(c_x86.contains("namespace aheadlibex {"));
    assert!(c_x86.contains("StringCchPrintfA"));
    assert!(c_x86.contains(r#"pfnAheadLibEx__Func__YAXH_Z = get_address("?Func@@YAXH@Z");"#));
    assert!(c_x86.contains(r#"pfnAheadLibEx__Func_8 = get_address("@Func@8");"#));
    assert!(c_x86.contains(r#"pfnAheadLibEx___0Class__QAE_XZ = get_address("??0Class@@QAE@XZ");"#));
    assert!(c_x86.contains("pfnAheadLibEx_Unnamed345 = get_address(MAKEINTRESOURCEA(345));"));

    let asm_x86 = render_asm_x86(&ctx);
    assert!(asm_x86.contains("PUBLIC AheadLibEx__Func__YAXH_Z"));
    assert!(asm_x86.contains("PUBLIC _AheadLibEx__Func__YAXH_Z"));

    let c_x64 = render_c_x64(&ctx);
    assert!(c_x64.contains("ScopedHandle"));
    assert!(c_x64.contains(r#"pfnAheadLibEx__Func__YAXH_Z = get_address("?Func@@YAXH@Z");"#));
    assert!(c_x64.contains(r#"pfnAheadLibEx__Func_8 = get_address("@Func@8");"#));
    assert!(c_x64.contains(r#"pfnAheadLibEx___0Class__QAE_XZ = get_address("??0Class@@QAE@XZ");"#));
    assert!(c_x64.contains("pfnAheadLibEx_Unnamed345 = get_address(MAKEINTRESOURCEA(345));"));

    let def_x86 = render_def(&ctx, false);
    assert!(def_x86.contains("\"?Func@@YAXH@Z\"=_AheadLibEx__Func__YAXH_Z @1"));
    assert!(def_x86.contains("\"@Func@8\"=_AheadLibEx__Func_8 @2"));
    assert!(def_x86.contains("\"??0Class@@QAE@XZ\"=_AheadLibEx___0Class__QAE_XZ @3"));
    assert!(def_x86.contains("Noname345=_AheadLibEx_Unnamed345 @345 NONAME"));

    let def_x64 = render_def(&ctx, true);
    assert!(def_x64.contains("\"?Func@@YAXH@Z\"=AheadLibEx__Func__YAXH_Z @1"));
    assert!(def_x64.contains("\"@Func@8\"=AheadLibEx__Func_8 @2"));
    assert!(def_x64.contains("\"??0Class@@QAE@XZ\"=AheadLibEx___0Class__QAE_XZ @3"));
    assert!(def_x64.contains("Noname345=AheadLibEx_Unnamed345 @345 NONAME"));
}

#[test]
fn asm_uses_sanitized_stub_names() {
    let exports = vec![named_export("?Decorated@Name@@@", 7)];
    let ctx = dummy_ctx(&exports);

    let asm = render_asm_x64(&ctx);
    assert!(asm.contains("EXTERN pfnAheadLibEx__Decorated_Name___:dq"));
    assert!(asm.contains("AheadLibEx__Decorated_Name___ PROC"));

    let asm_gas = render_asm_x64_gas(&ctx);
    assert!(asm_gas.contains(".extern pfnAheadLibEx__Decorated_Name___"));
    assert!(asm_gas.contains("AheadLibEx__Decorated_Name___:"));
}

#[test]
fn cmake_template_references_expected_files() {
    let exports = vec![named_export("Foo", 1)];
    let ctx = dummy_ctx(&exports);

    let cmake_x86 = render_cmake_lists(&ctx, false);
    assert!(cmake_x86.contains("project(AheadLibEx_Foo"));
    assert!(cmake_x86.contains("project(AheadLibEx_Foo LANGUAGES CXX)"));
    assert!(cmake_x86.contains("set(CMAKE_CXX_STANDARD 17)"));
    assert!(cmake_x86.contains("set(AHEADLIBEX_CPP \"Foo_x86.cpp\")"));
    assert!(cmake_x86.contains("set(AHEADLIBEX_ASM_MASM \"Foo_x86_jump.asm\")"));
    assert!(cmake_x86.contains("set(AHEADLIBEX_ASM_GAS \"Foo_x86_jump.S\")"));
    assert!(cmake_x86.contains("set(AHEADLIBEX_DEF \"Foo.def\")"));
    assert!(cmake_x86.contains("/DEF:${CMAKE_CURRENT_LIST_DIR}/${AHEADLIBEX_DEF}"));
}

#[test]
fn win32_vcxproj_enables_masm_safeseh() {
    let exports = vec![named_export("Foo", 1)];
    let ctx = dummy_ctx(&exports);

    let win32 = render_vcxproj(&ctx, false);
    assert!(win32.contains("<ClCompile Include=\"Foo_x86.cpp\" />"));
    assert!(win32.contains("<None Include=\"Foo.def\" />"));
    assert!(win32.contains("<ModuleDefinitionFile>Foo.def</ModuleDefinitionFile>"));
    assert!(win32.contains("<LanguageStandard>stdcpp17</LanguageStandard>"));
    assert!(win32.contains("<UseSafeExceptionHandlers>true</UseSafeExceptionHandlers>"));
    assert!(!win32.contains("/SAFESEH:NO"));

    let x64 = render_vcxproj(&ctx, true);
    assert!(x64.contains("<ClCompile Include=\"Foo_x64.cpp\" />"));
    assert!(!x64.contains("UseSafeExceptionHandlers"));
}

#[test]
fn hash_prefixed_and_spaced_names_are_treated_as_named_exports() {
    let exports = vec![
        named_export("#RealName", 9),
        named_export("Name With Space", 10),
        ordinal_only_export(11),
    ];
    let ctx = dummy_ctx(&exports);

    let c_x64 = render_c_x64(&ctx);
    assert!(c_x64.contains(r##"pfnAheadLibEx__RealName = get_address("#RealName");"##));
    assert!(c_x64.contains(r#"pfnAheadLibEx_Name_With_Space = get_address("Name With Space");"#));
    assert!(c_x64.contains("pfnAheadLibEx_Unnamed11 = get_address(MAKEINTRESOURCEA(11));"));

    let def_x64 = render_def(&ctx, true);
    assert!(def_x64.contains("\"#RealName\"=AheadLibEx__RealName @9"));
    assert!(def_x64.contains("\"Name With Space\"=AheadLibEx_Name_With_Space @10"));
    assert!(def_x64.contains("Noname11=AheadLibEx_Unnamed11 @11 NONAME"));
}
