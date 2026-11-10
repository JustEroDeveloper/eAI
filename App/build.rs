use std::path::PathBuf;

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="eAI.Lite" version="1.0.0.0" processorArchitecture="*"/>
  <description>eAI Lite</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#;

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn main() {
    println!("cargo:rerun-if-changed=assets/eai-lite.ico");
    println!("cargo:rerun-if-changed=build.rs");

    // Use the *target* OS. `cfg!(target_os = "windows")` checks the machine
    // running the build script, which skips resources when cross-compiling.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/eai-lite.ico");
    res.set_language(0x0409);
    res.set_manifest(MANIFEST);
    res.set("FileDescription", "eAI Lite");
    res.set("ProductName", "eAI Lite");
    res.set("OriginalFilename", "eai-lite.exe");
    res.set("InternalName", "eai-lite");
    res.set("CompanyName", "eAI");
    res.set("LegalCopyright", "eAI");
    res.set("FileVersion", "1.0.0.0");
    res.set("ProductVersion", "1.0.0.0");
    res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0001_0000_0000_0000);
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001_0000_0000_0000);

    // On a cross build the compiler is prefixed (e.g. x86_64-w64-mingw32-windres)
    // while winres looks for plain `windres` by default.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let names = match arch.as_str() {
            "x86_64" => vec!["x86_64-w64-mingw32-windres", "windres"],
            "aarch64" => vec!["aarch64-w64-mingw32-windres", "windres"],
            "i686" => vec!["i686-w64-mingw32-windres", "windres"],
            _ => vec!["windres"],
        };
        let from_env = std::env::var("WINDRES").ok().map(PathBuf::from);
        if let Some(path) = from_env.or_else(|| names.iter().find_map(|n| find_in_path(n))) {
            if let Some(path) = path.to_str() {
                res.set_windres_path(path);
            }
        }
    }

    if let Err(err) = res.compile() {
        println!("cargo:warning=could not embed Windows icon/resources: {err}");
    }

    // winres hands GNU ld a static archive containing only resources. Archives
    // are only searched for symbols, and resources have none, so the linker
    // silently drops the icon. Pass the object file to the final link instead.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let obj = PathBuf::from(out_dir).join("resource.o");
            if obj.exists() {
                println!("cargo:rustc-link-arg={}", obj.display());
            }
        }
    }
}
