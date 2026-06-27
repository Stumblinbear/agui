use std::cell::{Cell, RefCell};
use std::rc::Rc;

use agui::scheduling::TaskHandle;
use agui_test::{ElementTester, fixtures::Leaf};

#[test]
fn a_spawned_task_posts_a_message_back_to_its_element() {
    let received = Rc::new(Cell::new(None));
    let recorder = Rc::clone(&received);

    let widget = Leaf::new()
        .on_mount({
            // The element holds the task handle for its own lifetime, as a real element would; dropping
            // it would cancel the task.
            let handle: RefCell<Option<TaskHandle>> = RefCell::new(None);
            move |ctx| {
                *handle.borrow_mut() = Some(
                    ctx.spawn(|task| async move { task.send(42u32) })
                        .expect("scheduler available during mount"),
                );
            }
        })
        .on_message(move |ctx| recorder.set(Some(ctx.consume::<u32>())));

    let mut tester = ElementTester::mount(widget);
    assert_eq!(received.get(), None);

    tester.run_tasks();
    assert_eq!(received.get(), Some(42));
}
