#[macro_export]
macro_rules! select_template {
    ($hxboosted:expr, $hxrequest:expr, $language:expr) => {
        select_template!("alert", $hxboosted, $hxrequest, $language)
    };
    ($template_name:expr, $hxboosted:expr, $hxrequest:expr, $language:expr) => {{
        if $hxboosted {
            format!(
                concat!($template_name, ".{}.bare.html"),
                $language.to_string().to_lowercase()
            )
        } else if $hxrequest {
            format!(
                concat!($template_name, ".{}.partial.html"),
                $language.to_string().to_lowercase()
            )
        } else {
            format!(
                concat!($template_name, ".{}.html"),
                $language.to_string().to_lowercase()
            )
        }
    }};
}

#[macro_export]
macro_rules! contextual_error {
    ($web_context:expr, $language:expr, $template:expr, $template_context:expr, $error:expr) => {
        contextual_error!(
            $web_context,
            $language,
            $template,
            $template_context,
            $error,
            http::StatusCode::OK
        )
    };
    ($web_context:expr, $language:expr, $template:expr, $template_context:expr, $error:expr, $status_code:expr) => {
        {
            let (err_bare, err_partial) = $crate::errors::expand_error($error.to_string());
            tracing::warn!(error = ?$error, "encountered error");
            let error_message =
                $web_context
                    .i18n_context
                    .locales
                    .format_error(&$language, &err_bare, &err_partial);
            Ok(
                (
                    $status_code,
                    axum_template::RenderHtml(
                        &$template,
                        $web_context.engine.clone(),
                        minijinja::context! { ..$template_context, ..minijinja::context! {
                            message => error_message,
                        }},
                    )
                ).into_response()
            )
        }
    };
}
