use aheadlibex_rs::dll::ExportEntry;
use aheadlibex_rs::templates::{
    render_c, render_c_x64, OriginLoadMode, VsGuids, VsTemplateContext,
};

fn named_export(name: &str, ordinal: u16) -> ExportEntry {
    ExportEntry::named(name, ordinal, None)
}

fn dummy_ctx<'a>(exports: &'a [ExportEntry], mode: OriginLoadMode<'a>) -> VsTemplateContext<'a> {
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
        origin_load_mode: mode,
        exports,
        guids,
    }
}

#[test]
fn system_dir_mode_uses_system_directory() {
    let exports = vec![named_export("Bar", 1)];
    let ctx = dummy_ctx(&exports, OriginLoadMode::SystemDir);

    let c = render_c(&ctx);
    assert!(c.contains("namespace aheadlibex {"));
    assert!(c.contains("std::array<TCHAR, MAX_PATH>"));
    assert!(c.contains("load_original_module(HMODULE module)"));
    assert!(c.contains("load_original_module(module)"));
    assert!(c.contains("GetSystemDirectory("));
    assert!(c.contains(r#"TEXT("Foo.dll")"#));
}

#[test]
fn same_dir_mode_uses_proxy_directory_and_original_name() {
    let exports = vec![named_export("Bar", 1)];
    let ctx = dummy_ctx(
        &exports,
        OriginLoadMode::SameDir {
            original_name: "Foo_orig.dll",
        },
    );

    let c = render_c(&ctx);
    assert!(c.contains("query_module_relative_path(module, TEXT(\"Foo_orig.dll\"), module_path)"));
    assert!(c.contains("GetModuleFileName("));
    assert!(c.contains(r#"TEXT("Foo_orig.dll")"#));
}

#[test]
fn same_dir_mode_accepts_relative_subdirectory_path() {
    let exports = vec![named_export("Bar", 1)];
    let ctx = dummy_ctx(
        &exports,
        OriginLoadMode::SameDir {
            original_name: r".\bar\Foo_orig.dll",
        },
    );

    let c = render_c_x64(&ctx);
    assert!(c.contains(r#"TEXT(".\\bar\\Foo_orig.dll")"#));
    assert!(c.contains(
        "query_module_relative_path(module, TEXT(\".\\\\bar\\\\Foo_orig.dll\"), module_path)"
    ));
}

#[test]
fn custom_path_mode_embeds_origin_cfg() {
    let exports = vec![named_export("Bar", 1)];
    let ctx = dummy_ctx(
        &exports,
        OriginLoadMode::CustomPath {
            path: r"C:\path\to\Foo.dll",
        },
    );

    let c = render_c_x64(&ctx);
    assert!(c.contains(r#"origin_cfg[] = TEXT("C:\\path\\to\\Foo.dll")"#));
    assert!(c.contains("is_absolute_path(origin_cfg)"));
    assert!(c.contains("copy_text(module_path, origin_cfg)"));
}

#[test]
fn custom_relative_path_mode_uses_proxy_directory() {
    let exports = vec![named_export("Bar", 1)];
    let ctx = dummy_ctx(
        &exports,
        OriginLoadMode::CustomPath {
            path: r".\bar\Foo.dll",
        },
    );

    let c = render_c_x64(&ctx);
    assert!(c.contains(r#"origin_cfg[] = TEXT(".\\bar\\Foo.dll")"#));
    assert!(c.contains("query_module_relative_path(module, origin_cfg, module_path)"));
    assert!(c.contains("is_absolute_path(origin_cfg)"));
}
