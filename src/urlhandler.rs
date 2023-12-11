use std::io;
use std::env::current_exe;
use const_format::concatcp;
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const CANONICAL_NAME: &str = "browserselect.exe";
const PROGID: &str = "BrowserSelector";

// Configuration for "Default Programs". StartMenuInternet is the key for browsers
// and they're expected to use the name of the exe as the key.
const DPROG_PATH: &str = concatcp!(r"SOFTWARE\Clients\StartMenuInternet\", CANONICAL_NAME);
//const DPROG_INSTALLINFO_PATH: &str = concatcp!(DPROG_PATH, "InstallInfo");

const APPREG_BASE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\";
const PROGID_PATH: &str = concatcp!(r"SOFTWARE\Classes\", PROGID);
//const REGISTERED_APPLICATIONS_PATH: &str = concatcp!(r"SOFTWARE\RegisteredApplications\", DISPLAY_NAME);

const DISPLAY_NAME: &str = "browserselect";
const DESCRIPTION: &str = "Pick the right browser for different url patterns";

/// Register associations with Windows for being a browser
pub fn register() -> io::Result<()> {
    // This is used both by initial registration and OS-invoked reinstallation.
    // The expectations for the latter are documented here: https://docs.microsoft.com/en-us/windows/win32/shell/reg-middleware-apps#the-reinstall-command

    let exe_path = current_exe()?;
    let exe_name = exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();

    let exe_path = exe_path.to_str().unwrap_or_default().to_owned();
    let icon_path = format!("\"{}\",0", exe_path);
    let open_command = format!("\"{}\" \"%1\"", exe_path);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Configure our ProgID to point to the right command
    {
        let (progid_class, _) = hkcu.create_subkey(PROGID_PATH)?;
        progid_class.set_value("", &DISPLAY_NAME)?;

        let (progid_class_defaulticon, _) = progid_class.create_subkey("DefaultIcon")?;
        progid_class_defaulticon.set_value("", &icon_path)?;

        let (progid_class_shell_open_command, _) =
            progid_class.create_subkey(r"shell\open\command")?;
        progid_class_shell_open_command.set_value("", &open_command)?;
    }

    // Set up the Default Programs configuration for the app (https://docs.microsoft.com/en-us/windows/win32/shell/default-programs)
    {
        let (dprog, _) = hkcu.create_subkey(DPROG_PATH)?;
        dprog.set_value("", &DISPLAY_NAME)?;
        dprog.set_value("LocalizedString", &DISPLAY_NAME)?;

        let (dprog_capabilites, _) = dprog.create_subkey("Capabilities")?;
        dprog_capabilites.set_value("ApplicationName", &DISPLAY_NAME)?;
        dprog_capabilites.set_value("ApplicationIcon", &icon_path)?;
        dprog_capabilites.set_value("ApplicationDescription", &DESCRIPTION)?;

        let (dprog_capabilities_startmenu, _) = dprog_capabilites.create_subkey("Startmenu")?;
        dprog_capabilities_startmenu.set_value("StartMenuInternet", &CANONICAL_NAME)?;

        // Register for various URL protocols that our target browsers might support.
        // (The list of protocols that Chrome registers for is actually quite large, including irc, mailto, mms,
        // etc, but let's do the most obvious/significant ones.)
        let (dprog_capabilities_urlassociations, _) =
            dprog_capabilites.create_subkey("URLAssociations")?;
        for protocol in &["bichrome", "ftp", "http", "https", "webcal"] {
            dprog_capabilities_urlassociations.set_value(protocol, &PROGID)?;
        }

        // Register for various file types, so that we'll be invoked for file:// URLs for these types (e.g.
        // by `cargo doc --open`.)
        let (dprog_capabilities_fileassociations, _) =
            dprog_capabilites.create_subkey("FileAssociations")?;
        for filetype in &[
            ".htm", ".html", ".pdf", ".shtml", ".svg", ".webp", ".xht", ".xhtml",
            ] {
            dprog_capabilities_fileassociations.set_value(filetype, &PROGID)?;
        }

        let (dprog_defaulticon, _) = dprog.create_subkey("DefaultIcon")?;
        dprog_defaulticon.set_value("", &icon_path)?;

        // Set up reinstallation and show/hide icon commands (https://docs.microsoft.com/en-us/windows/win32/shell/reg-middleware-apps#registering-installation-information)
        let (dprog_installinfo, _) = dprog.create_subkey("InstallInfo")?;
        dprog_installinfo.set_value("ReinstallCommand", &format!("\"{}\" register", exe_path))?;
        dprog_installinfo.set_value("HideIconsCommand", &format!("\"{}\" hide-icons", exe_path))?;
        dprog_installinfo.set_value("ShowIconsCommand", &format!("\"{}\" show-icons", exe_path))?;

        // Only update IconsVisible if it hasn't been set already
        if dprog_installinfo
            .get_value::<u32, _>("IconsVisible")
            .is_err()
        {
            dprog_installinfo.set_value("IconsVisible", &1u32)?;
        }

        let (dprog_shell_open_command, _) = dprog.create_subkey(r"shell\open\command")?;
        dprog_shell_open_command.set_value("", &open_command)?;
    }

    // Set up a registered application for our Default Programs capabilities (https://docs.microsoft.com/en-us/windows/win32/shell/default-programs#registeredapplications)
    {
        let (registered_applications, _) =
            hkcu.create_subkey(r"SOFTWARE\RegisteredApplications")?;
        let dprog_capabilities_path = format!(r"{}\Capabilities", DPROG_PATH);
        registered_applications.set_value(DISPLAY_NAME, &dprog_capabilities_path)?;
    }

    // Application Registration (https://docs.microsoft.com/en-us/windows/win32/shell/app-registration)
    {
        let appreg_path = format!(r"{}{}", APPREG_BASE, exe_name);
        let (appreg, _) = hkcu.create_subkey(appreg_path)?;
        // This is used to resolve "bichrome.exe" -> full path, if needed.
        appreg.set_value("", &exe_path)?;
        // UseUrl indicates that we don't need the shell to download a file for us -- we can handle direct
        // HTTP URLs.
        appreg.set_value("UseUrl", &1u32)?;
    }

    // refresh_shell();

    Ok(())
}

/*
fn refresh_shell() {
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_DWORD, SHCNF_FLUSH};

    // Notify the shell about the updated URL associations. (https://docs.microsoft.com/en-us/windows/win32/shell/default-programs#becoming-the-default-browser)
    unsafe {
        SHChangeNotify(
                SHCNE_ASSOCCHANGED,
            SHCNF_DWORD | SHCNF_FLUSH,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
}*/
