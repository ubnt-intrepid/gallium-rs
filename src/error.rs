error_chain! {
    types {
        AppError, AppErrorKind, ResultExt, AppResult;
    }

    foreign_links {
        Io(::std::io::Error);
        Diesel(::diesel::result::Error);
        R2D2Initialization(::r2d2::InitializationError);
        R2D2(::r2d2::GetTimeout);
        Git2(::git2::Error);
        SerdeJson(::serde_json::Error);
        JsonWebToken(::jsonwebtoken::errors::Error);
        Bcrypt(::bcrypt::BcryptError);
    }
}
