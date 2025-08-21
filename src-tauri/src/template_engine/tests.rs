use super::*;
use chrono::Local;
use mockito::mock;

#[tokio::test]
async fn test_parse_current_date() {
    let template_engine = TemplateEngine::new();
    let result = template_engine.parse("test !current_date test", &HashMap::new()).await;
    println!("result : {}", result);
    assert_eq!(result.contains(&Local::now().format("%Y-%m-%d").to_string()), true);

    let result2 = template_engine.parse("test !cd test", &HashMap::new()).await;
    println!("result2 : {}", result2);
    assert_eq!(result2.contains(&Local::now().format("%Y-%m-%d").to_string()), true);

    let result3 = template_engine.parse("test!cd()test", &HashMap::new()).await;
    println!("result3 : {}", result3);
    assert_eq!(result3.contains(&Local::now().format("%Y-%m-%d").to_string()), true);
}

#[tokio::test]
async fn test_parse_sub_start() {
    let template_engine = TemplateEngine::new();
    let result =
        template_engine.parse("test !sub_start(123123123123,3) test", &HashMap::new()).await;
    println!("result : {}", result);
    assert_eq!(result, "test 123 test");

    let result2 = template_engine
        .parse("test !sub_start(asfdsafd 123123,asfasf,123123,12) test", &HashMap::new())
        .await;
    println!("result2 : {}", result2);
    assert_eq!(result2, "test asfdsafd 123 test");
}

#[tokio::test]
async fn test_parse_selected_text() {
    let template_engine = TemplateEngine::new();
    let mut context = HashMap::new();
    context.insert("selected_text".to_string(), "test".to_string());
    let result = template_engine.parse("test !selected_text test", &context).await;
    println!("result : {}", result);
    assert_eq!(result, "test test test");

    let result2 = template_engine.parse("test !s test", &context).await;
    println!("result2 : {}", result2);
    assert_eq!(result2, "test test test");
}

#[tokio::test]
async fn test_parse_mix() {
    let template_engine = TemplateEngine::new();
    let mut context = HashMap::new();
    context.insert("selected_text".to_string(), "test".to_string());
    let result = template_engine.parse("test!sub_start(!selected_text,2)test", &context).await;
    println!("result : {}", result);
    assert_eq!(result, "testtetest");

    let result2 = template_engine.parse("test!sub_start(!cd,4)test", &context).await;
    println!("result2 : {}", result2);
    assert_eq!(result2.contains(&Local::now().format("%Y").to_string()), true);

    let result3 = template_engine.parse("test!sub_start(!sub_start(!cd,4),2)test", &context).await;
    println!("result3 : {}", result3);
    assert_eq!("test20test", result3);
}

#[tokio::test]
async fn test_web_command() {
    let html_content = "<html><body><h1>Hello, World!</h1><p>This is a test.</p></body></html>";
    let mock = mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(html_content)
        .expect(3)
        .create();

    // 先确认mock server是否正常
    let client = reqwest::Client::new();
    match client.get(&mockito::server_url()).send().await {
        Ok(response) => {
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers().get("content-type").unwrap(), "text/html");

            let body = response.text().await.unwrap();
            println!("test case body : {}", body);
            assert_eq!(body, html_content);
        }
        Err(err) => {
            panic!("Request failed: {}", err);
        }
    }

    let engine = TemplateEngine::new();
    let context = HashMap::new();

    let except_result =
        format!("\n<bangweb url=\"{}\">\n{}\n</bangweb>", &mockito::server_url(), html_content);

    let result = engine.parse(&format!("!web({})", mockito::server_url()), &context).await;
    assert_eq!(result, except_result);

    let result = engine.parse(&format!("!w({})", mockito::server_url()), &context).await;
    assert_eq!(result, except_result);

    mock.assert();
}

#[tokio::test]
async fn test_web_to_markdown_command() {
    let html_content = "<html><body><h1>Hello, World!</h1><p>This is a test.</p></body></html>";
    let mock = mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(html_content)
        .expect(3)
        .create();

    // 先确认mock server是否正常
    let client = reqwest::Client::new();
    match client.get(&mockito::server_url()).send().await {
        Ok(response) => {
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers().get("content-type").unwrap(), "text/html");

            let body = response.text().await.unwrap();
            println!("test case body : {}", body);
            assert_eq!(body, html_content);
        }
        Err(err) => {
            panic!("Request failed: {}", err);
        }
    }

    let engine = TemplateEngine::new();
    let context = HashMap::new();

    let except_result = format!(
        "\n<bangwebtomarkdown url=\"{}\">\n{}\n</bangwebtomarkdown>",
        &mockito::server_url(),
        "# Hello, World!\n\nThis is a test."
    );

    let result =
        engine.parse(&format!("!web_to_markdown({})", mockito::server_url()), &context).await;
    assert_eq!(result, except_result);

    let result = engine.parse(&format!("!wm({})", mockito::server_url()), &context).await;
    assert_eq!(result, except_result);

    mock.assert();
}
