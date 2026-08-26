//! 跨平台文件权限与属性兼容层。
//!
//! 在 Unix / Linux 下调用原生 PermissionsExt 设置 0o600 / 0o700 / 0o755 权限及执行位；
//! 在 Windows 下提供安全的 no-op 与兼容回退，避免跨平台编译断裂。

use std::fs::{DirBuilder, Metadata, OpenOptions, Permissions};
use std::io::Result;
use std::path::Path;

/// 设置文件/目录权限位（Unix 下生效，Windows 下 no-op）。
pub fn set_file_mode<P: AsRef<Path>>(path: P, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path.as_ref(), Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}

/// 构造指定模式的 Permissions（Unix 下生效，Windows 下返回只读/读写默认权限）。
pub fn permissions_from_mode(mode: u32) -> Permissions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Permissions::from_mode(mode)
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        std::fs::metadata(".").map(|m| m.permissions()).unwrap_or_else(|_| {
            // 回退到临时文件权限探测
            std::env::temp_dir().metadata().unwrap().permissions()
        })
    }
}

/// 为文件添加可执行权限（Unix 下生效，Windows 下 no-op）。
pub fn make_executable<P: AsRef<Path>>(path: P) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path.as_ref())?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path.as_ref(), perms)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// 获取文件权限模式数值（Unix 下返回原生 mode，Windows 下返回 0o644）。
pub fn get_file_mode(metadata: &Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        0o644
    }
}

/// 跨平台 OpenOptions 扩展 trait。
pub trait OpenOptionsExtCompat {
    fn set_mode(&mut self, mode: u32) -> &mut Self;
}

impl OpenOptionsExtCompat for OpenOptions {
    fn set_mode(&mut self, mode: u32) -> &mut Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            self.mode(mode);
        }
        #[cfg(not(unix))]
        {
            let _ = mode;
        }
        self
    }
}

/// 跨平台 DirBuilder 扩展 trait。
pub trait DirBuilderExtCompat {
    fn set_mode(&mut self, mode: u32) -> &mut Self;
}

impl DirBuilderExtCompat for DirBuilder {
    fn set_mode(&mut self, mode: u32) -> &mut Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            self.mode(mode);
        }
        #[cfg(not(unix))]
        {
            let _ = mode;
        }
        self
    }
}
