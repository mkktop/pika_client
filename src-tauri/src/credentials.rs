//! 系统凭据库封装：把哔咔账号密码存进 Windows 凭据管理器 / macOS 钥匙串，
//! 供 token 过期时自动重新登录。密码绝不写入 sqlite 明文。

use keyring::Entry;

const SERVICE: &str = "com.pika.client";

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, account).map_err(|e| format!("无法访问系统凭据库：{e}"))
}

/// 保存密码。失败时返回错误文案，由调用方决定是否提示（登录本身不受影响）。
pub fn save(account: &str, password: &str) -> Result<(), String> {
    entry(account)?
        .set_password(password)
        .map_err(|e| format!("保存凭据失败：{e}"))
}

/// 读取密码；条目不存在返回 Ok(None)，其余错误返回 Err。
pub fn load(account: &str) -> Result<Option<String>, String> {
    match entry(account)?.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("读取凭据失败：{e}")),
    }
}

/// 删除凭据；条目本就不存在时视为成功。
pub fn delete(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("清除凭据失败：{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_entry_roundtrip_or_env_error() {
        // Windows/macOS 本地凭据库在测试机上都可用；CI 无凭据库时 keyring 返回平台错误，
        // 此时跳过断言而不是让测试假失败。
        let account = format!("pika-test-{}", uuid::Uuid::new_v4());
        let Ok(entry) = Entry::new(SERVICE, &account) else {
            return;
        };
        if entry.set_password("s3cret").is_err() {
            return;
        }
        assert_eq!(load(&account).unwrap().as_deref(), Some("s3cret"));
        delete(&account).unwrap();
        assert_eq!(load(&account).unwrap(), None);
        // 重复删除应幂等。
        delete(&account).unwrap();
    }
}
