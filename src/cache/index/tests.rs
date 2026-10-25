//! The index hook alone: the no-op one, and a recording one read back
//! through `&dyn Any` as `src/search` will read its own.

use std::any::Any;

use super::{Index, Rowless};
use crate::cache::tests::{Recorder, Test, row};

#[test]
fn rowless_keeps_nothing_and_holds_no_bytes() -> Test {
    let mut none = Rowless;
    let o = row(1, "a")?;
    none.insert(&(1, o.id.clone()), &o, Some(&o));
    none.reset();
    assert_eq!(none.bytes(), 0);
    Ok(())
}

#[test]
fn an_index_reads_back_as_its_own_type() -> Test {
    let mut recorder = Recorder::new(10);
    let (a, b) = (row(1, "a")?, row(1, "b")?);
    let key = (1, a.id.clone());
    recorder.insert(&key, &a, None);
    recorder.insert(&key, &b, Some(&a));
    let index: &dyn Index = &recorder;
    let any: &dyn Any = index;
    let back = any.downcast_ref::<Recorder>().ok_or("not a recorder")?;
    assert_eq!(back.bytes(), 10);
    assert_eq!(back.replaced, ["a"]);
    recorder.reset();
    assert_eq!((recorder.bytes(), recorder.resets), (0, 1));
    Ok(())
}
