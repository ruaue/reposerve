mod args;
mod config;
mod auth;

use crate::{
	args::Opts,
	config::Config,
	auth::TokenAuth
};

use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{
	App,
	HttpServer,
	HttpResponse,
	Error,
	web, post,
	middleware::Logger
};
use futures::{
	StreamExt,
	TryStreamExt,
};
use std::io::Write;

#[post("/upload")]
async fn save_file(mut payload: Multipart) -> Result<HttpResponse, Error> {
	// iterate over multipart stream
	while let Ok(Some(mut field)) = payload.try_next().await {
		let content_type = field.content_disposition().unwrap();
		let filename = content_type.get_filename().unwrap();
		let filepath = format!("./tmp/{}", sanitize_filename::sanitize(&filename));

		// File::create is blocking operation, use threadpool
		let mut f = web::block(|| std::fs::File::create(filepath))
			.await
			.unwrap();

		// Field in turn is stream of *Bytes* object
		while let Some(chunk) = field.next().await {
			let data = chunk.unwrap();
			// filesystem operations are blocking, we have to use threadpool
			f = web::block(move || f.write_all(&data).map(|_| f)).await?;
		}
	}
	Ok(HttpResponse::Ok().into())
}

#[actix_web::main]
async fn serve(config: Config) -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    HttpServer::new(move || {
		App::new()
			.wrap(Logger::default())
			.wrap(TokenAuth)
			.data(config.clone())
			.service(save_file)
			.service(Files::new("/", ".").show_files_listing())
	})
	.bind("127.0.0.1:8080")?
	.run()
	.await
}

fn main() -> anyhow::Result<()> {
	let opts: Opts = argh::from_env();
	// read yaml config
	let config = Config::read(&opts.config)?;
	serve(config).unwrap();
	Ok(())
}
