use chrono::{DateTime, Duration, Utc};
use lambda_runtime::{handler_fn, Context};
use rusoto_cloudwatch::{
    CloudWatch, CloudWatchClient, Dimension, GetMetricStatisticsError, GetMetricStatisticsInput,
    GetMetricStatisticsOutput,
};
use rusoto_core::{Region, RusotoError};
use serde_json::{json, Value};
use simplelog::{Config, LevelFilter};
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    simplelog::SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let func = handler_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(_: Value, _: Context) -> Result<Value, lambda_runtime::Error> {
    let statistics = get_estimated_charges_from_cloudwatch().await?;
    let attachment = build_message(statistics);
    match post_message_to_slack(attachment).await {
        Ok(v) => Ok(v),
        Err(e) => Err(e),
    }
}

async fn post_message_to_slack(attachment: Value) -> Result<Value, lambda_runtime::Error> {
    let slack_post_url = env::var("SLACK_POST_URL").unwrap();
    let slack_channel = env::var("SLACK_CHANNEL").unwrap();
    log::info!(
        "start: slackPostURL:{}, slackChannel:{}",
        slack_post_url,
        slack_channel
    );

    let body = json!({
        "channel": slack_channel.to_string(),
        "attachments": vec![attachment],
    });

    let client = reqwest::Client::new();
    client
        .post(slack_post_url.to_string())
        .body(body.to_string())
        .send()
        .await?;

    Ok(json!({
        "message": format!("Send message to {}{}", slack_post_url, slack_channel)
    }))
}

fn build_message(statistics: GetMetricStatisticsOutput) -> Value {
    let points = statistics.datapoints.unwrap();
    let dp = points.get(0).unwrap();

    let date = DateTime::parse_from_rfc3339(dp.timestamp.as_ref().unwrap().as_str())
        .unwrap()
        .format("%Y年%m月%d日")
        .to_string();
    let cost = dp.maximum.unwrap_or(0.0);

    let color = match cost {
        n if (n < 0.5) => "good",               // green
        n if (0.5 < n && n < 1.0) => "warning", // yellow
        _ => "#ff0000",                         // red
    };

    let text = format!("{}までのAWSの料金は、${}です。", date, cost);
    return json!({
        "text": text,
        "color": color,
    });
}

async fn get_estimated_charges_from_cloudwatch(
) -> Result<GetMetricStatisticsOutput, RusotoError<GetMetricStatisticsError>> {
    let client = CloudWatchClient::new(Region::UsEast1);

    client
        .get_metric_statistics(GetMetricStatisticsInput {
            namespace: String::from("AWS/Billing"),
            metric_name: String::from("EstimatedCharges"),

            dimensions: Some(vec![Dimension {
                name: String::from("Currency"),
                value: String::from("USD"),
            }]),

            start_time: (Utc::today().and_hms(0, 0, 0) - Duration::days(1))
                .format("%+")
                .to_string(),
            end_time: Utc::today().and_hms(0, 0, 0).format("%+").to_string(),

            period: 86400,
            statistics: Some(vec![String::from("Maximum")]),
            ..GetMetricStatisticsInput::default()
        })
        .await
}
