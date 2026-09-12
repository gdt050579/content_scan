use crate::MyTypes;
use content_scan::*;

#[derive(Dependencies)]
#[Dependencies(name = "DotNetAnalyzer")]
pub struct DotNetAnalyzer;

impl ContentAnalyzer<MyTypes> for DotNetAnalyzer {
    fn analyze(&mut self, _: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> AnalysisOutcome<MyTypes> {
        context.add_finding("clr:runtime=v4", Some("DotNetAnalyzer"), None);
        AnalysisOutcome::Continue
    }
}
