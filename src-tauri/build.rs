fn main() {
    tauri_build::build();

    // 单元测试 exe 也需要嵌入应用清单（tauri_build 生成的 libresource.a，
    // 内含 comctl32 v6 依赖声明）：tauri-build 默认只对 bin 目标链接该资源，
    // 而测试二进制一旦链接进 tao/comctl32 相关符号（如 TaskDialogIndirect），
    // 缺清单会在进程加载期报 0xc0000139 (STATUS_ENTRYPOINT_NOT_FOUND)。
    // 这里对 test 目标补链同一资源。OUT_DIR 即 libresource.a 所在目录。
    #[cfg(windows)]
    {
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR 未设置");
        let resource = std::path::Path::new(&out_dir).join("libresource.a");
        if resource.exists() {
            // 注意用 rustc-link-arg（对 bins/tests/examples 统一生效）而非
            // rustc-link-arg-tests（要求包显式声明 [[test]] 目标）。
            // bins 已获 tauri-build 注入的 rustc-link-arg-bins，重复链接同一资源
            // 在 GNU ld 下幂等（同一 .a 仅取一次成员），不会重复嵌入清单。
            println!("cargo:rustc-link-arg={}", resource.display());
        }
    }
}
