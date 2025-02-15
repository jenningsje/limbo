use crate::errors::Result;
use crate::errors::{LimboError, LIMBO_ETC};
use crate::utils::set_err_msg_and_throw_exception;
use jni::objects::{JObject, JValue};
use jni::sys::jlong;
use jni::JNIEnv;
use limbo_core::{Statement, StepResult};

pub const STEP_RESULT_ID_ROW: i32 = 10;
pub const STEP_RESULT_ID_IO: i32 = 20;
pub const STEP_RESULT_ID_DONE: i32 = 30;
pub const STEP_RESULT_ID_INTERRUPT: i32 = 40;
pub const STEP_RESULT_ID_BUSY: i32 = 50;
pub const STEP_RESULT_ID_ERROR: i32 = 60;

pub struct LimboStatement {
    pub(crate) stmt: Statement,
}

impl LimboStatement {
    pub fn new(stmt: Statement) -> Self {
        LimboStatement { stmt }
    }

    pub fn to_ptr(self) -> jlong {
        Box::into_raw(Box::new(self)) as jlong
    }

    #[allow(dead_code)]
    pub fn drop(ptr: jlong) {
        let _boxed = unsafe { Box::from_raw(ptr as *mut LimboStatement) };
    }
}

pub fn to_limbo_statement(ptr: jlong) -> Result<&'static mut LimboStatement> {
    if ptr == 0 {
        Err(LimboError::InvalidConnectionPointer)
    } else {
        unsafe { Ok(&mut *(ptr as *mut LimboStatement)) }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_github_tursodatabase_core_LimboStatement_step<'local>(
    mut env: JNIEnv<'local>,
    obj: JObject<'local>,
    stmt_ptr: jlong,
) -> JObject<'local> {
    let stmt = match to_limbo_statement(stmt_ptr) {
        Ok(stmt) => stmt,
        Err(e) => {
            set_err_msg_and_throw_exception(&mut env, obj, LIMBO_ETC, e.to_string());

            return JObject::null();
        }
    };

    match stmt.stmt.step() {
        Ok(StepResult::Row(row)) => match row_to_obj_array(&mut env, &row) {
            Ok(row) => to_limbo_step_result(&mut env, STEP_RESULT_ID_ROW, Some(row)),
            Err(e) => {
                set_err_msg_and_throw_exception(&mut env, obj, LIMBO_ETC, e.to_string());
                to_limbo_step_result(&mut env, STEP_RESULT_ID_ERROR, None)
            }
        },
        Ok(StepResult::IO) => match env.new_object_array(0, "java/lang/Object", JObject::null()) {
            Ok(row) => to_limbo_step_result(&mut env, STEP_RESULT_ID_IO, Some(row.into())),
            Err(e) => {
                set_err_msg_and_throw_exception(&mut env, obj, LIMBO_ETC, e.to_string());
                to_limbo_step_result(&mut env, STEP_RESULT_ID_ERROR, None)
            }
        },
        Ok(StepResult::Done) => to_limbo_step_result(&mut env, STEP_RESULT_ID_DONE, None),
        Ok(StepResult::Interrupt) => to_limbo_step_result(&mut env, STEP_RESULT_ID_INTERRUPT, None),
        Ok(StepResult::Busy) => to_limbo_step_result(&mut env, STEP_RESULT_ID_BUSY, None),
        _ => to_limbo_step_result(&mut env, STEP_RESULT_ID_ERROR, None),
    }
}

fn row_to_obj_array<'local>(
    env: &mut JNIEnv<'local>,
    row: &limbo_core::Row,
) -> Result<JObject<'local>> {
    let obj_array =
        env.new_object_array(row.values.len() as i32, "java/lang/Object", JObject::null())?;

    for (i, value) in row.values.iter().enumerate() {
        let obj = match value {
            limbo_core::Value::Null => JObject::null(),
            limbo_core::Value::Integer(i) => {
                env.new_object("java/lang/Long", "(J)V", &[JValue::Long(*i)])?
            }
            limbo_core::Value::Float(f) => {
                env.new_object("java/lang/Double", "(D)V", &[JValue::Double(*f)])?
            }
            limbo_core::Value::Text(s) => env.new_string(s)?.into(),
            limbo_core::Value::Blob(b) => env.byte_array_from_slice(b)?.into(),
        };
        if let Err(e) = env.set_object_array_element(&obj_array, i as i32, obj) {
            eprintln!("Error on parsing row: {:?}", e);
        }
    }

    Ok(obj_array.into())
}

/// Converts an optional `JObject` into Java's `LimboStepResult`.
///
/// This function takes an optional `JObject` and converts it into a Java object
/// of type `LimboStepResult`. The conversion is done by creating a new Java object with the
/// appropriate constructor arguments.
///
/// # Arguments
///
/// * `env` - A mutable reference to the JNI environment.
/// * `id` - An integer representing the type of `StepResult`.
/// * `result` - An optional `JObject` that contains the result data.
///
/// # Returns
///
/// A `JObject` representing the `LimboStepResult` in Java. If the object creation fails,
/// a null `JObject` is returned
fn to_limbo_step_result<'local>(
    env: &mut JNIEnv<'local>,
    id: i32,
    result: Option<JObject<'local>>,
) -> JObject<'local> {
    let mut ctor_args = vec![JValue::Int(id)];
    if let Some(res) = result {
        ctor_args.push(JValue::Object(&res));
        env.new_object(
            "org/github/tursodatabase/core/LimboStepResult",
            "(I[Ljava/lang/Object;)V",
            &ctor_args,
        )
    } else {
        env.new_object(
            "org/github/tursodatabase/core/LimboStepResult",
            "(I)V",
            &ctor_args,
        )
    }
    .unwrap_or_else(|_| JObject::null())
}
