use minicbor::Encode;
use problem_details::ProblemDetail;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Never(std::convert::Infallible);

impl ProblemDetail for Never {
    fn slug(&self) -> &'static str {
        match self.0 {}
    }
    fn problem_type(&self) -> &'static str {
        match self.0 {}
    }
    fn title(&self) -> &'static str {
        match self.0 {}
    }
    fn http_status(&self) -> u16 {
        match self.0 {}
    }
}

impl Serialize for Never {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match self.0 {}
    }
}

impl Encode<()> for Never {
    fn encode<W: minicbor::encode::Write>(
        &self,
        _: &mut minicbor::Encoder<W>,
        _: &mut (),
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self.0 {}
    }
}
