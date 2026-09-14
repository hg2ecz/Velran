pub(crate) const SOURCE: &str = r#"
#[allow(dead_code)]
mod velran_numeric {
use super::velran_support::{as_f32, as_int, OutputWriter, RuntimeState, Scalar};
use std::fmt::{self, Write};
use std::sync::OnceLock;
use std::time::Instant;
const MAX_F32_ARRAY_ELEMENTS: usize = 1_048_576;

pub(crate) fn typed_f32_finite(value: f32) -> Option<f32> { value.is_finite().then_some(value) }
pub(crate) fn typed_f32_sqrt(value: f32) -> Option<f32> { if value < 0.0 { None } else { typed_f32_finite(value.sqrt()) } }
pub(crate) fn typed_to_f32(value: i64) -> Option<f32> { typed_f32_finite(value as f32) }
pub(crate) fn typed_monotonic_nanos() -> i64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_nanos().min(i64::MAX as u128) as i64
}
pub(crate) fn typed_f32_array_new(len: Option<i64>, fill: Option<f32>, state: &mut RuntimeState) -> Option<Vec<f32>> {
    let len = usize::try_from(len?).ok()?;
    let fill = fill?;
    if !fill.is_finite() { state.bad_request(); return None; }
    if len > MAX_F32_ARRAY_ELEMENTS { state.bad_request(); return None; }
    let bytes = len.checked_mul(std::mem::size_of::<f32>())?.saturating_add(24) as u64;
    if !state.charge_alloc(bytes) { return None; }
    let mut values = Vec::new();
    if values.try_reserve_exact(len).is_err() { state.memory_exceeded(); return None; }
    values.resize(len, fill);
    Some(values)
}
pub(crate) fn typed_f32_array_get(array: &[f32], index: Option<i64>, state: &mut RuntimeState) -> Option<f32> {
    let Ok(index) = usize::try_from(index?) else { state.bad_request(); return None; };
    match array.get(index).copied() { Some(value) => Some(value), None => { state.bad_request(); None } }
}
pub(crate) fn typed_f32_array_set(array: &mut [f32], index: Option<i64>, value: Option<f32>, state: &mut RuntimeState) -> bool {
    let Ok(index) = usize::try_from(match index { Some(v) => v, None => { state.bad_request(); return false; } }) else { state.bad_request(); return false; };
    let Some(value) = value.filter(|v| v.is_finite()) else { state.bad_request(); return false; };
    let Some(slot) = array.get_mut(index) else { state.bad_request(); return false; };
    *slot = value; true
}

pub(crate) fn f32_array_new(len: Option<Scalar>, fill: Option<Scalar>, state: &mut RuntimeState) -> Option<Scalar> {
    let len = usize::try_from(as_int(len?)?).ok()?;
    let fill = as_f32(fill?)?;
    if len > MAX_F32_ARRAY_ELEMENTS || !fill.is_finite() { state.bad_request(); return None; }
    let bytes = len.checked_mul(std::mem::size_of::<f32>())?.saturating_add(24) as u64;
    if !state.charge_alloc(bytes) { return None; }
    let mut values = Vec::new();
    if values.try_reserve_exact(len).is_err() { state.memory_exceeded(); return None; }
    values.resize(len, fill);
    Some(Scalar::F32Array(values))
}
pub(crate) fn f32_array_set(array: &mut Scalar, index: Option<Scalar>, value: Option<Scalar>, state: &mut RuntimeState) -> bool {
    let Ok(index) = usize::try_from(match index.and_then(as_int) { Some(v) => v, None => { state.bad_request(); return false; } }) else { state.bad_request(); return false; };
    let Some(value) = value.and_then(as_f32).filter(|v| v.is_finite()) else { state.bad_request(); return false; };
    let Scalar::F32Array(items) = array else { state.bad_request(); return false; };
    let Some(slot) = items.get_mut(index) else { state.bad_request(); return false; };
    *slot = value; true
}
fn finite(value: f32) -> Option<Scalar> { value.is_finite().then_some(Scalar::F32(value)) }
pub(crate) fn numeric_add(a: Scalar, b: Scalar) -> Option<Scalar> { numeric(a, b, i64::checked_add, |a,b| a + b) }
pub(crate) fn numeric_sub(a: Scalar, b: Scalar) -> Option<Scalar> { numeric(a, b, i64::checked_sub, |a,b| a - b) }
pub(crate) fn numeric_mul(a: Scalar, b: Scalar) -> Option<Scalar> { numeric(a, b, i64::checked_mul, |a,b| a * b) }
pub(crate) fn numeric_div(a: Scalar, b: Scalar) -> Option<Scalar> { numeric(a, b, i64::checked_div, |a,b| a / b) }
pub(crate) fn numeric_rem(a: Scalar, b: Scalar) -> Option<Scalar> { numeric(a, b, i64::checked_rem, |a,b| a % b) }
fn numeric(a: Scalar, b: Scalar, int_op: fn(i64,i64)->Option<i64>, f32_op: fn(f32,f32)->f32) -> Option<Scalar> {
    match (a,b) { (Scalar::Int(a), Scalar::Int(b)) => int_op(a,b).map(Scalar::Int), (Scalar::F32(a), Scalar::F32(b)) => finite(f32_op(a,b)), _ => None }
}
pub(crate) fn numeric_lt(a: Option<Scalar>, b: Option<Scalar>) -> Option<Scalar> { numeric_cmp(a,b,|a,b| a<b,|a,b| a<b) }
pub(crate) fn numeric_le(a: Option<Scalar>, b: Option<Scalar>) -> Option<Scalar> { numeric_cmp(a,b,|a,b| a<=b,|a,b| a<=b) }
pub(crate) fn numeric_gt(a: Option<Scalar>, b: Option<Scalar>) -> Option<Scalar> { numeric_cmp(a,b,|a,b| a>b,|a,b| a>b) }
pub(crate) fn numeric_ge(a: Option<Scalar>, b: Option<Scalar>) -> Option<Scalar> { numeric_cmp(a,b,|a,b| a>=b,|a,b| a>=b) }
fn numeric_cmp(a: Option<Scalar>, b: Option<Scalar>, int_op: fn(i64,i64)->bool, f32_op: fn(f32,f32)->bool) -> Option<Scalar> {
    match (a?,b?) { (Scalar::Int(a),Scalar::Int(b))=>Some(Scalar::Bool(int_op(a,b))), (Scalar::F32(a),Scalar::F32(b))=>Some(Scalar::Bool(f32_op(a,b))), _=>None }
}
pub(crate) fn f32_unary(value: Option<Scalar>, op: fn(f32)->f32) -> Option<Scalar> { finite(op(as_f32(value?)?)) }
pub(crate) fn f32_sqrt(value: Option<Scalar>) -> Option<Scalar> { let value = as_f32(value?)?; if value < 0.0 { None } else { finite(value.sqrt()) } }
pub(crate) fn to_f32(value: Option<Scalar>) -> Option<Scalar> { finite(as_int(value?)? as f32) }
pub(crate) fn monotonic_nanos() -> Option<Scalar> { Some(Scalar::Int(typed_monotonic_nanos())) }
struct StackFmt<'a> { buf: &'a mut [u8], len: usize }
impl fmt::Write for StackFmt<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result { let end = self.len.checked_add(text.len()).ok_or(fmt::Error)?; if end > self.buf.len() { return Err(fmt::Error); } self.buf[self.len..end].copy_from_slice(text.as_bytes()); self.len=end; Ok(()) }
}
pub(crate) fn push_f32(value: f32, out: &mut OutputWriter<'_>, state: &mut RuntimeState) -> bool {
    if !value.is_finite() { state.bad_request(); return false; }
    let mut buf=[0u8;64]; let mut fmt=StackFmt { buf:&mut buf, len:0 };
    if write!(&mut fmt, "{value}").is_err() { state.bad_request(); return false; }
    let text=match std::str::from_utf8(&fmt.buf[..fmt.len]) { Ok(v)=>v, Err(_)=>return false };
    out.push(text,state)
}
}
"#;
