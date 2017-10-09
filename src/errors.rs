error_chain! {
    foreign_links {
        ParseInt(::std::num::ParseIntError);
    }
    errors {
        InvalidMouseEventSpec(e: String) {
            description("provided mouse event specification is not valid")
            display("mouse event specification {} is not valid", e)
        }
        InvalidKeyboardEventSpec(e: String) {
            description("provided keyboard event specification is not valid")
            display("keyboard event specification {} is not valid", e)
        }
    }
}
