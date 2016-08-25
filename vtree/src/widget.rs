use group::Group;
use std::fmt::Debug;

pub trait WidgetDataTrait<O>: Debug
	where O: Group
{
	fn render(self: Box<Self>) -> Option<O>;
	fn clone_box(&self) -> Box<WidgetDataTrait<O>>;
}

impl<O> Clone for Box<WidgetDataTrait<O>>
	where O: Group
{
	fn clone(&self) -> Box<WidgetDataTrait<O>> {
		self.clone_box()
	}
}

#[derive(Debug, Clone)]
pub struct WidgetData<W: Widget> (pub W::Input);

impl<O, W> WidgetDataTrait<O> for WidgetData<W>
	where
		O: Group,
		W: Widget<Output = O> + 'static
{
	fn render(self: Box<Self>) -> Option<O> {
		W::new().render(self.0)
	}

	fn clone_box(&self) -> Box<WidgetDataTrait<O>> {
		Box::new((*self).clone())
	}
}

pub trait Widget: Debug + Clone {
	type Input: Debug + Clone;
	type Output: Group;

	fn new() -> Self;
	fn render(&self, Self::Input) -> Option<Self::Output>;
}
