use std::fmt::Debug;

pub trait WidgetDataTrait<O>: Debug
    where O: Debug + Clone
{
    fn render(self: Box<Self>) -> Option<O>;
    fn clone_box(&self) -> Box<WidgetDataTrait<O>>;
}

impl<O> Clone for Box<WidgetDataTrait<O>>
    where O: Debug + Clone
{
    fn clone(&self) -> Box<WidgetDataTrait<O>> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct WidgetData<W: Widget>(pub W::Input);

impl<O, W> WidgetDataTrait<O> for WidgetData<W>
    where O: Debug + Clone,
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
    type Output: Debug + Clone;

    fn new() -> Self;
    fn render(&self, Self::Input) -> Option<Self::Output>;
}


#[derive(Debug, Clone)]
pub struct NullWidgetData;

impl<O> WidgetDataTrait<O> for NullWidgetData
    where O: Debug + Clone
{
    fn render(self: Box<Self>) -> Option<O> {
        panic!("rendering NullWidgetData");
    }

    fn clone_box(&self) -> Box<WidgetDataTrait<O>> {
        Box::new((*self).clone())
    }
}
