pub fn websocket_origin() -> String {
    let location = web_sys::window().unwrap().location();
    let proto = location.protocol().unwrap();
    let host = location.host().unwrap();

    // let host = host.replace(":8080", ":3000");

    let proto = match proto.as_str() {
        "https:" => "wss:",
        _ => "ws:",
    };

    format!("{}//{}", proto, host)
}
