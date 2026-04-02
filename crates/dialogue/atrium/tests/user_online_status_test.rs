mod fixtures;

use atrium::db::user_manager::CreateUserDto;
use fixtures::{create_user_manager, setup_test_db};

/// 测试单用户在线状态
#[tokio::test]
async fn test_online_status_single_user() {
    let (_container, db) = setup_test_db().await;
    let manager = create_user_manager(&db);

    // 创建用户
    let dto = CreateUserDto {
        name: "alice".to_string(),
        bio: "Alice's bio".to_string(),
        password: "pass".to_string(),
    };
    manager.create_user(&dto).await.unwrap();

    // 无 heartbeat 时离线
    let user = manager.get_user_by_name("alice").await.unwrap();
    assert!(!user.status.online);
    assert!(user.status.last_seen.is_none());

    // heartbeat 后在线
    let user = manager.update_heartbeat("alice").await.unwrap();
    assert!(user.status.online);
    assert!(user.status.last_seen.is_some());

    // heartbeat 刷新时间
    let first_seen = user.status.last_seen.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let user = manager.update_heartbeat("alice").await.unwrap();
    let second_seen = user.status.last_seen.unwrap();
    assert!(second_seen >= first_seen);
}

/// 测试多用户在线状态
#[tokio::test]
async fn test_online_status_multiple_users() {
    let (_container, db) = setup_test_db().await;
    let manager = create_user_manager(&db);

    // 创建三个用户
    for name in ["alice", "bob", "charlie"] {
        manager
            .create_user(&CreateUserDto {
                name: name.to_string(),
                bio: format!("{}'s bio", name),
                password: "pass".to_string(),
            })
            .await
            .unwrap();
    }

    // 只有 alice 发送 heartbeat
    manager.update_heartbeat("alice").await.unwrap();

    // 验证各用户状态
    let users = manager.get_all_users().await.unwrap();
    assert_eq!(users.len(), 3);

    for user in users {
        match user.name.as_str() {
            "alice" => {
                assert!(user.status.online);
                assert!(user.status.last_seen.is_some());
            }
            _ => {
                assert!(!user.status.online);
                assert!(user.status.last_seen.is_none());
            }
        }
    }
}
