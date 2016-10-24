#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

extern {
    fn cargo_apk_injected_glue_jni_attach_thread() -> *mut c_void;
    fn cargo_apk_injected_glue_jni_detach_thread();
    fn cargo_apk_injected_glue_jni_class_loader() -> *mut c_void;
    fn cargo_apk_injected_glue_jni_activity() -> *mut c_void;
}

use std::os::raw::c_void;
use std::os::raw::c_float;
use std::os::raw::c_double;
use std::os::raw::c_char;
use std::os::raw::c_schar;
use std::os::raw::c_uchar;
use std::os::raw::c_int;
use std::os::raw::c_short;
use std::os::raw::c_ushort;
use std::os::raw::c_longlong;
use std::ptr;
use std::cell::RefCell;
use std::ffi::CString;

// Class types
pub type class__jarray = ();
pub type class__jbooleanArray = ();
pub type class__jbyteArray = ();
pub type class__jcharArray = ();
pub type class__jclass = ();
pub type class__jdoubleArray = ();
pub type class__jfloatArray = ();
pub type class__jintArray = ();
pub type class__jlongArray = ();
pub type class__jobject = ();
pub type class__jobjectArray = ();
pub type class__jshortArray = ();
pub type class__jstring = ();
pub type class__jthrowable = ();

// Non-class types
pub type jarray = *mut class__jarray;
pub type jboolean = c_uchar;
pub type jbooleanArray = *mut class__jbooleanArray;
pub type jbyte = c_schar;
pub type jbyteArray = *mut class__jbyteArray;
pub type jchar = c_ushort;
pub type jcharArray = *mut class__jcharArray;
pub type jclass = *mut class__jclass;
pub type jdouble = c_double;
pub type jdoubleArray = *mut class__jdoubleArray;
pub type jfieldID = *mut _jfieldID;
pub type jfloat = c_float;
pub type jfloatArray = *mut class__jfloatArray;
pub type jint = c_int;
pub type jintArray = *mut class__jintArray;
pub type jlong = c_longlong;
pub type jlongArray = *mut class__jlongArray;
pub type jmethodID = *mut _jmethodID;
pub type jobject = *mut class__jobject;
pub type jobjectArray = *mut class__jobjectArray;
pub type jobjectRefType = i32;
pub type jshort = c_short;
pub type jshortArray = *mut class__jshortArray;
pub type jsize = jint;
pub type jstring = *mut class__jstring;
pub type jthrowable = *mut class__jthrowable;
pub type jvalue = [u8; 8];
pub type jweak = *mut class__jobject;

// JNI types
pub type _jfieldID = ();
pub type _jmethodID = ();
pub type __va_list_tag = ();
pub type JavaVM = ();
#[repr(C)]
pub struct _JNIEnv {
     pub functions:             *const JNINativeInterface,
}
pub type JNIEnv = *mut _JNIEnv;
//pub struct JNIEnv {
//    pub raw_pointer: *mut _JNIEnv,
//    pub class_loader: jobject,
//}

#[repr(C)]
pub struct JNINativeInterface {
    pub reserved0:             *mut c_void,
    pub reserved1:             *mut c_void,
    pub reserved2:             *mut c_void,
    pub reserved3:             *mut c_void,
    pub GetVersion:                extern fn(JNIEnv) -> jint,
    pub DefineClass:               extern fn(JNIEnv, *const c_char, jobject, *const jbyte, jsize) -> jclass,
    pub FindClass:             extern fn(JNIEnv, *const c_char) -> jclass,
    pub FromReflectedMethod:               extern fn(JNIEnv, jobject) -> jmethodID,
    pub FromReflectedField:                extern fn(JNIEnv, jobject) -> jfieldID,
    pub ToReflectedMethod:             extern fn(JNIEnv, jclass, jmethodID, jboolean) -> jobject,
    pub GetSuperclass:             extern fn(JNIEnv, jclass) -> jclass,
    pub IsAssignableFrom:              extern fn(JNIEnv, jclass, jclass) -> jboolean,
    pub ToReflectedField:              extern fn(JNIEnv, jclass, jfieldID, jboolean) -> jobject,
    pub Throw:             extern fn(JNIEnv, jthrowable) -> jint,
    pub ThrowNew:              extern fn(JNIEnv, jclass, *const c_char) -> jint,
    pub ExceptionOccurred:             extern fn(JNIEnv) -> jthrowable,
    pub ExceptionDescribe:             extern fn(JNIEnv),
    pub ExceptionClear:                extern fn(JNIEnv),
    pub FatalError:                extern fn(JNIEnv, *const c_char),
    pub PushLocalFrame:                extern fn(JNIEnv, jint) -> jint,
    pub PopLocalFrame:             extern fn(JNIEnv, jobject) -> jobject,
    pub NewGlobalRef:              extern fn(JNIEnv, jobject) -> jobject,
    pub DeleteGlobalRef:               extern fn(JNIEnv, jobject),
    pub DeleteLocalRef:                extern fn(JNIEnv, jobject),
    pub IsSameObject:              extern fn(JNIEnv, jobject, jobject) -> jboolean,
    pub NewLocalRef:               extern fn(JNIEnv, jobject) -> jobject,
    pub EnsureLocalCapacity:               extern fn(JNIEnv, jint) -> jint,
    pub AllocObject:               extern fn(JNIEnv, jclass) -> jobject,
    pub NewObject:             extern fn(JNIEnv, jclass, jmethodID, ...) -> jobject,
    pub NewObjectV:                extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jobject,
    pub NewObjectA:                extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jobject,
    pub GetObjectClass:                extern fn(JNIEnv, jobject) -> jclass,
    pub IsInstanceOf:              extern fn(JNIEnv, jobject, jclass) -> jboolean,
    pub GetMethodID:               extern fn(JNIEnv, jclass, *const c_char, *const c_char) -> jmethodID,
    pub CallObjectMethod:              extern fn(JNIEnv, jobject, jmethodID, ...) -> jobject,
    pub CallObjectMethodV:             extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jobject,
    pub CallObjectMethodA:             extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jobject,
    pub CallBooleanMethod:             extern fn(JNIEnv, jobject, jmethodID, ...) -> jboolean,
    pub CallBooleanMethodV:                extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jboolean,
    pub CallBooleanMethodA:                extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jboolean,
    pub CallByteMethod:                extern fn(JNIEnv, jobject, jmethodID, ...) -> jbyte,
    pub CallByteMethodV:               extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jbyte,
    pub CallByteMethodA:               extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jbyte,
    pub CallCharMethod:                extern fn(JNIEnv, jobject, jmethodID, ...) -> jchar,
    pub CallCharMethodV:               extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jchar,
    pub CallCharMethodA:               extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jchar,
    pub CallShortMethod:               extern fn(JNIEnv, jobject, jmethodID, ...) -> jshort,
    pub CallShortMethodV:              extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jshort,
    pub CallShortMethodA:              extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jshort,
    pub CallIntMethod:             extern fn(JNIEnv, jobject, jmethodID, ...) -> jint,
    pub CallIntMethodV:                extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jint,
    pub CallIntMethodA:                extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jint,
    pub CallLongMethod:                extern fn(JNIEnv, jobject, jmethodID, ...) -> jlong,
    pub CallLongMethodV:               extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jlong,
    pub CallLongMethodA:               extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jlong,
    pub CallFloatMethod:               extern fn(JNIEnv, jobject, jmethodID, ...) -> jfloat,
    pub CallFloatMethodV:              extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jfloat,
    pub CallFloatMethodA:              extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jfloat,
    pub CallDoubleMethod:              extern fn(JNIEnv, jobject, jmethodID, ...) -> jdouble,
    pub CallDoubleMethodV:             extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag) -> jdouble,
    pub CallDoubleMethodA:             extern fn(JNIEnv, jobject, jmethodID, *mut jvalue) -> jdouble,
    pub CallVoidMethod:                extern fn(JNIEnv, jobject, jmethodID, ...),
    pub CallVoidMethodV:               extern fn(JNIEnv, jobject, jmethodID, *mut __va_list_tag),
    pub CallVoidMethodA:               extern fn(JNIEnv, jobject, jmethodID, *mut jvalue),
    pub CallNonvirtualObjectMethod:                extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jobject,
    pub CallNonvirtualObjectMethodV:               extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jobject,
    pub CallNonvirtualObjectMethodA:               extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jobject,
    pub CallNonvirtualBooleanMethod:               extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jboolean,
    pub CallNonvirtualBooleanMethodV:              extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jboolean,
    pub CallNonvirtualBooleanMethodA:              extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jboolean,
    pub CallNonvirtualByteMethod:              extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jbyte,
    pub CallNonvirtualByteMethodV:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jbyte,
    pub CallNonvirtualByteMethodA:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jbyte,
    pub CallNonvirtualCharMethod:              extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jchar,
    pub CallNonvirtualCharMethodV:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jchar,
    pub CallNonvirtualCharMethodA:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jchar,
    pub CallNonvirtualShortMethod:             extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jshort,
    pub CallNonvirtualShortMethodV:                extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jshort,
    pub CallNonvirtualShortMethodA:                extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jshort,
    pub CallNonvirtualIntMethod:               extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jint,
    pub CallNonvirtualIntMethodV:              extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jint,
    pub CallNonvirtualIntMethodA:              extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jint,
    pub CallNonvirtualLongMethod:              extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jlong,
    pub CallNonvirtualLongMethodV:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jlong,
    pub CallNonvirtualLongMethodA:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jlong,
    pub CallNonvirtualFloatMethod:             extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jfloat,
    pub CallNonvirtualFloatMethodV:                extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jfloat,
    pub CallNonvirtualFloatMethodA:                extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jfloat,
    pub CallNonvirtualDoubleMethod:                extern fn(JNIEnv, jobject, jclass, jmethodID, ...) -> jdouble,
    pub CallNonvirtualDoubleMethodV:               extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag) -> jdouble,
    pub CallNonvirtualDoubleMethodA:               extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue) -> jdouble,
    pub CallNonvirtualVoidMethod:              extern fn(JNIEnv, jobject, jclass, jmethodID, ...),
    pub CallNonvirtualVoidMethodV:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut __va_list_tag),
    pub CallNonvirtualVoidMethodA:             extern fn(JNIEnv, jobject, jclass, jmethodID, *mut jvalue),
    pub GetFieldID:                extern fn(JNIEnv, jclass, *const c_char, *const c_char) -> jfieldID,
    pub GetObjectField:                extern fn(JNIEnv, jobject, jfieldID) -> jobject,
    pub GetBooleanField:               extern fn(JNIEnv, jobject, jfieldID) -> jboolean,
    pub GetByteField:              extern fn(JNIEnv, jobject, jfieldID) -> jbyte,
    pub GetCharField:              extern fn(JNIEnv, jobject, jfieldID) -> jchar,
    pub GetShortField:             extern fn(JNIEnv, jobject, jfieldID) -> jshort,
    pub GetIntField:               extern fn(JNIEnv, jobject, jfieldID) -> jint,
    pub GetLongField:              extern fn(JNIEnv, jobject, jfieldID) -> jlong,
    pub GetFloatField:             extern fn(JNIEnv, jobject, jfieldID) -> jfloat,
    pub GetDoubleField:                extern fn(JNIEnv, jobject, jfieldID) -> jdouble,
    pub SetObjectField:                extern fn(JNIEnv, jobject, jfieldID, jobject),
    pub SetBooleanField:               extern fn(JNIEnv, jobject, jfieldID, jboolean),
    pub SetByteField:              extern fn(JNIEnv, jobject, jfieldID, jbyte),
    pub SetCharField:              extern fn(JNIEnv, jobject, jfieldID, jchar),
    pub SetShortField:             extern fn(JNIEnv, jobject, jfieldID, jshort),
    pub SetIntField:               extern fn(JNIEnv, jobject, jfieldID, jint),
    pub SetLongField:              extern fn(JNIEnv, jobject, jfieldID, jlong),
    pub SetFloatField:             extern fn(JNIEnv, jobject, jfieldID, jfloat),
    pub SetDoubleField:                extern fn(JNIEnv, jobject, jfieldID, jdouble),
    pub GetStaticMethodID:             extern fn(JNIEnv, jclass, *const c_char, *const c_char) -> jmethodID,
    pub CallStaticObjectMethod:                extern fn(JNIEnv, jclass, jmethodID, ...) -> jobject,
    pub CallStaticObjectMethodV:               extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jobject,
    pub CallStaticObjectMethodA:               extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jobject,
    pub CallStaticBooleanMethod:               extern fn(JNIEnv, jclass, jmethodID, ...) -> jboolean,
    pub CallStaticBooleanMethodV:              extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jboolean,
    pub CallStaticBooleanMethodA:              extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jboolean,
    pub CallStaticByteMethod:              extern fn(JNIEnv, jclass, jmethodID, ...) -> jbyte,
    pub CallStaticByteMethodV:             extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jbyte,
    pub CallStaticByteMethodA:             extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jbyte,
    pub CallStaticCharMethod:              extern fn(JNIEnv, jclass, jmethodID, ...) -> jchar,
    pub CallStaticCharMethodV:             extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jchar,
    pub CallStaticCharMethodA:             extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jchar,
    pub CallStaticShortMethod:             extern fn(JNIEnv, jclass, jmethodID, ...) -> jshort,
    pub CallStaticShortMethodV:                extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jshort,
    pub CallStaticShortMethodA:                extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jshort,
    pub CallStaticIntMethod:               extern fn(JNIEnv, jclass, jmethodID, ...) -> jint,
    pub CallStaticIntMethodV:              extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jint,
    pub CallStaticIntMethodA:              extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jint,
    pub CallStaticLongMethod:              extern fn(JNIEnv, jclass, jmethodID, ...) -> jlong,
    pub CallStaticLongMethodV:             extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jlong,
    pub CallStaticLongMethodA:             extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jlong,
    pub CallStaticFloatMethod:             extern fn(JNIEnv, jclass, jmethodID, ...) -> jfloat,
    pub CallStaticFloatMethodV:                extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jfloat,
    pub CallStaticFloatMethodA:                extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jfloat,
    pub CallStaticDoubleMethod:                extern fn(JNIEnv, jclass, jmethodID, ...) -> jdouble,
    pub CallStaticDoubleMethodV:               extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag) -> jdouble,
    pub CallStaticDoubleMethodA:               extern fn(JNIEnv, jclass, jmethodID, *mut jvalue) -> jdouble,
    pub CallStaticVoidMethod:              extern fn(JNIEnv, jclass, jmethodID, ...),
    pub CallStaticVoidMethodV:             extern fn(JNIEnv, jclass, jmethodID, *mut __va_list_tag),
    pub CallStaticVoidMethodA:             extern fn(JNIEnv, jclass, jmethodID, *mut jvalue),
    pub GetStaticFieldID:              extern fn(JNIEnv, jclass, *const c_char, *const c_char) -> jfieldID,
    pub GetStaticObjectField:              extern fn(JNIEnv, jclass, jfieldID) -> jobject,
    pub GetStaticBooleanField:             extern fn(JNIEnv, jclass, jfieldID) -> jboolean,
    pub GetStaticByteField:                extern fn(JNIEnv, jclass, jfieldID) -> jbyte,
    pub GetStaticCharField:                extern fn(JNIEnv, jclass, jfieldID) -> jchar,
    pub GetStaticShortField:               extern fn(JNIEnv, jclass, jfieldID) -> jshort,
    pub GetStaticIntField:             extern fn(JNIEnv, jclass, jfieldID) -> jint,
    pub GetStaticLongField:                extern fn(JNIEnv, jclass, jfieldID) -> jlong,
    pub GetStaticFloatField:               extern fn(JNIEnv, jclass, jfieldID) -> jfloat,
    pub GetStaticDoubleField:              extern fn(JNIEnv, jclass, jfieldID) -> jdouble,
    pub SetStaticObjectField:              extern fn(JNIEnv, jclass, jfieldID, jobject),
    pub SetStaticBooleanField:             extern fn(JNIEnv, jclass, jfieldID, jboolean),
    pub SetStaticByteField:                extern fn(JNIEnv, jclass, jfieldID, jbyte),
    pub SetStaticCharField:                extern fn(JNIEnv, jclass, jfieldID, jchar),
    pub SetStaticShortField:               extern fn(JNIEnv, jclass, jfieldID, jshort),
    pub SetStaticIntField:             extern fn(JNIEnv, jclass, jfieldID, jint),
    pub SetStaticLongField:                extern fn(JNIEnv, jclass, jfieldID, jlong),
    pub SetStaticFloatField:               extern fn(JNIEnv, jclass, jfieldID, jfloat),
    pub SetStaticDoubleField:              extern fn(JNIEnv, jclass, jfieldID, jdouble),
    pub NewString:             extern fn(JNIEnv, *const jchar, jsize) -> jstring,
    pub GetStringLength:               extern fn(JNIEnv, jstring) -> jsize,
    pub GetStringChars:                extern fn(JNIEnv, jstring, *mut jboolean) -> *const jchar,
    pub ReleaseStringChars:                extern fn(JNIEnv, jstring, *const jchar),
    pub NewStringUTF:              extern fn(JNIEnv, *const c_char) -> jstring,
    pub GetStringUTFLength:                extern fn(JNIEnv, jstring) -> jsize,
    pub GetStringUTFChars:             extern fn(JNIEnv, jstring, *mut jboolean) -> *const c_char,
    pub ReleaseStringUTFChars:             extern fn(JNIEnv, jstring, *const c_char),
    pub GetArrayLength:                extern fn(JNIEnv, jarray) -> jsize,
    pub NewObjectArray:                extern fn(JNIEnv, jsize, jclass, jobject) -> jobjectArray,
    pub GetObjectArrayElement:             extern fn(JNIEnv, jobjectArray, jsize) -> jobject,
    pub SetObjectArrayElement:             extern fn(JNIEnv, jobjectArray, jsize, jobject),
    pub NewBooleanArray:               extern fn(JNIEnv, jsize) -> jbooleanArray,
    pub NewByteArray:              extern fn(JNIEnv, jsize) -> jbyteArray,
    pub NewCharArray:              extern fn(JNIEnv, jsize) -> jcharArray,
    pub NewShortArray:             extern fn(JNIEnv, jsize) -> jshortArray,
    pub NewIntArray:               extern fn(JNIEnv, jsize) -> jintArray,
    pub NewLongArray:              extern fn(JNIEnv, jsize) -> jlongArray,
    pub NewFloatArray:             extern fn(JNIEnv, jsize) -> jfloatArray,
    pub NewDoubleArray:                extern fn(JNIEnv, jsize) -> jdoubleArray,
    pub GetBooleanArrayElements:               extern fn(JNIEnv, jbooleanArray, *mut jboolean) -> *mut jboolean,
    pub GetByteArrayElements:              extern fn(JNIEnv, jbyteArray, *mut jboolean) -> *mut jbyte,
    pub GetCharArrayElements:              extern fn(JNIEnv, jcharArray, *mut jboolean) -> *mut jchar,
    pub GetShortArrayElements:             extern fn(JNIEnv, jshortArray, *mut jboolean) -> *mut jshort,
    pub GetIntArrayElements:               extern fn(JNIEnv, jintArray, *mut jboolean) -> *mut jint,
    pub GetLongArrayElements:              extern fn(JNIEnv, jlongArray, *mut jboolean) -> *mut jlong,
    pub GetFloatArrayElements:             extern fn(JNIEnv, jfloatArray, *mut jboolean) -> *mut jfloat,
    pub GetDoubleArrayElements:                extern fn(JNIEnv, jdoubleArray, *mut jboolean) -> *mut jdouble,
    pub ReleaseBooleanArrayElements:               extern fn(JNIEnv, jbooleanArray, *mut jboolean, jint),
    pub ReleaseByteArrayElements:              extern fn(JNIEnv, jbyteArray, *mut jbyte, jint),
    pub ReleaseCharArrayElements:              extern fn(JNIEnv, jcharArray, *mut jchar, jint),
    pub ReleaseShortArrayElements:             extern fn(JNIEnv, jshortArray, *mut jshort, jint),
    pub ReleaseIntArrayElements:               extern fn(JNIEnv, jintArray, *mut jint, jint),
    pub ReleaseLongArrayElements:              extern fn(JNIEnv, jlongArray, *mut jlong, jint),
    pub ReleaseFloatArrayElements:             extern fn(JNIEnv, jfloatArray, *mut jfloat, jint),
    pub ReleaseDoubleArrayElements:                extern fn(JNIEnv, jdoubleArray, *mut jdouble, jint),
    pub GetBooleanArrayRegion:             extern fn(JNIEnv, jbooleanArray, jsize, jsize, *mut jboolean),
    pub GetByteArrayRegion:                extern fn(JNIEnv, jbyteArray, jsize, jsize, *mut jbyte),
    pub GetCharArrayRegion:                extern fn(JNIEnv, jcharArray, jsize, jsize, *mut jchar),
    pub GetShortArrayRegion:               extern fn(JNIEnv, jshortArray, jsize, jsize, *mut jshort),
    pub GetIntArrayRegion:             extern fn(JNIEnv, jintArray, jsize, jsize, *mut jint),
    pub GetLongArrayRegion:                extern fn(JNIEnv, jlongArray, jsize, jsize, *mut jlong),
    pub GetFloatArrayRegion:               extern fn(JNIEnv, jfloatArray, jsize, jsize, *mut jfloat),
    pub GetDoubleArrayRegion:              extern fn(JNIEnv, jdoubleArray, jsize, jsize, *mut jdouble),
    pub SetBooleanArrayRegion:             extern fn(JNIEnv, jbooleanArray, jsize, jsize, *const jboolean),
    pub SetByteArrayRegion:                extern fn(JNIEnv, jbyteArray, jsize, jsize, *const jbyte),
    pub SetCharArrayRegion:                extern fn(JNIEnv, jcharArray, jsize, jsize, *const jchar),
    pub SetShortArrayRegion:               extern fn(JNIEnv, jshortArray, jsize, jsize, *const jshort),
    pub SetIntArrayRegion:             extern fn(JNIEnv, jintArray, jsize, jsize, *const jint),
    pub SetLongArrayRegion:                extern fn(JNIEnv, jlongArray, jsize, jsize, *const jlong),
    pub SetFloatArrayRegion:               extern fn(JNIEnv, jfloatArray, jsize, jsize, *const jfloat),
    pub SetDoubleArrayRegion:              extern fn(JNIEnv, jdoubleArray, jsize, jsize, *const jdouble),
    pub RegisterNatives:               extern fn(JNIEnv, jclass, *const JNINativeMethod, jint) -> jint,
    pub UnregisterNatives:             extern fn(JNIEnv, jclass) -> jint,
    pub MonitorEnter:              extern fn(JNIEnv, jobject) -> jint,
    pub MonitorExit:               extern fn(JNIEnv, jobject) -> jint,
    pub GetJavaVM:             extern fn(JNIEnv, *mut *mut JavaVM) -> jint,
    pub GetStringRegion:               extern fn(JNIEnv, jstring, jsize, jsize, *mut jchar),
    pub GetStringUTFRegion:                extern fn(JNIEnv, jstring, jsize, jsize, *mut c_char),
    pub GetPrimitiveArrayCritical:             extern fn(JNIEnv, jarray, *mut jboolean) -> *mut c_void,
    pub ReleasePrimitiveArrayCritical:             extern fn(JNIEnv, jarray, *mut c_void, jint),
    pub GetStringCritical:             extern fn(JNIEnv, jstring, *mut jboolean) -> *const jchar,
    pub ReleaseStringCritical:             extern fn(JNIEnv, jstring, *const jchar),
    pub NewWeakGlobalRef:              extern fn(JNIEnv, jobject) -> jweak,
    pub DeleteWeakGlobalRef:               extern fn(JNIEnv, jweak),
    pub ExceptionCheck:                extern fn(JNIEnv) -> jboolean,
    pub NewDirectByteBuffer:               extern fn(JNIEnv, *mut c_void, jlong) -> jobject,
    pub GetDirectBufferAddress:                extern fn(JNIEnv, jobject) -> *mut c_void,
    pub GetDirectBufferCapacity:               extern fn(JNIEnv, jobject) -> jlong,
    pub GetObjectRefType:              extern fn(JNIEnv, jobject) -> jobjectRefType,
}

#[repr(C)]
pub struct JNINativeMethod {
    pub name:              *const c_char,
    pub signature:         *const c_char,
    pub fnPtr:             *mut c_void,
}

struct JNIEnvHandle {
    pub jni_env: JNIEnv,
    pub counter: usize,
}

thread_local! {
    static JNI_HANDLE : RefCell<JNIEnvHandle> = RefCell::new(JNIEnvHandle { jni_env: ptr::null_mut(), counter: 0 });
}

pub fn attach_thread() -> JNIEnv {
    JNI_HANDLE.with( |jni_handle_cell| {
        let mut jni_handle = jni_handle_cell.borrow_mut();
        if jni_handle.counter == 0 {
            unsafe {
                jni_handle.jni_env = cargo_apk_injected_glue_jni_attach_thread() as *mut _;
            }
        }
        
        if jni_handle.jni_env.is_null() {
            panic!("Failed to attach thread to JVM!");
        }
        
        jni_handle.counter += 1;
        jni_handle.jni_env
    })
}

pub fn detach_thread() {
    JNI_HANDLE.with( |jni_handle_cell| {
        let mut jni_handle = jni_handle_cell.borrow_mut();
        if jni_handle.counter == 0 {
            return; // Nothing to do
        }
        
        jni_handle.counter -= 1;
        
        if jni_handle.counter == 0 {
            unsafe { cargo_apk_injected_glue_jni_detach_thread(); }
            jni_handle.jni_env = ptr::null_mut();
        }
    });
}

pub fn get_current_activity() -> jobject {
  let result = unsafe { cargo_apk_injected_glue_jni_activity() };
  if result.is_null() { panic!("Android Activity instance could not be retrieved."); }
  result as jobject
}

pub trait JNIWrappers {
    fn find_class(&self, name: &str) -> Option<jclass>;
    fn find_method(&self, class: jclass, name: &str, signature: &str) -> Option<jmethodID>;
    fn find_field(&self, class: jclass, name: &str, signature: &str) -> Option<jfieldID>;
    fn find_static_method(&self, class: jclass, name: &str, signature: &str) -> Option<jmethodID>;
    fn find_static_field(&self, class: jclass, name: &str, signature: &str) -> Option<jfieldID>;
    fn delete_local_ref(&self, obj: jobject);
    fn delete_global_ref(&self, obj: jobject);
    fn new_global_ref(&self, obj: jobject) -> Option<jobject>;
    fn get_object_class(&self, obj: jobject) -> jclass;
    fn instance_of(&self, obj: jobject, class: jclass) -> bool;
    fn has_exception(&self) -> bool;
    fn clear_exception(&self);
    fn describe_exception(&self);
    fn ensure_local_capacity(&self, size: i32) -> Option<()> ;
    fn functions<'a>(&'a self) -> &'a JNINativeInterface;
}

impl JNIWrappers for JNIEnv {
    fn find_class(&self, name: &str) -> Option<jclass> {
        let class_name = CString::new(name).unwrap();
        let mut class = unsafe { (self.functions().FindClass)(*self, class_name.as_ptr()) };
        
        if class.is_null() {
            println!("Class was not found using normal FindClass, falling back to ClassLoader method.");
            // Rust code is running in a different thread, created in android_main2.
            // As per https://developer.android.com/training/articles/perf-jni.html#faq_FindClass,
            // this can cause issues with classes not being found.
            // We will need to load the class ourself using the main threads' class loader.
            // Take care to check for any raised exception, appropriately returning None in such a case, and clearing the exception.
            unsafe {
                let class_name_loader_format = CString::new(String::from(name).replace("/", ".")).unwrap();
                
                let class_loader : jobject = cargo_apk_injected_glue_jni_class_loader() as *mut _;
                if class_loader.is_null() {
                    panic!("Android class loader could not be retrieved.")
                }
                
                let class_loader_class = self.get_object_class(class_loader);
                let load_class_method = self.find_method(class_loader_class, "loadClass", "(Ljava/lang/String;)Ljava/lang/Class;").unwrap();
                let class_name_str : jstring = (self.functions().NewStringUTF)(*self, class_name_loader_format.as_ptr());
                class = (self.functions().CallObjectMethod)(*self, class_loader, load_class_method, class_name_str);
                self.delete_local_ref(class_name_str);
                if self.has_exception() {
                    println!("Warning: exception raised during loadClass");
                    self.describe_exception();
                    self.clear_exception();
                    class = ptr::null_mut();
                }
            }
        }
        
        if class.is_null() {
            println!("Warning: failed to find class named {}", name);
            None
        } else {
            Some(class)
        }
    }
    
    fn find_method(&self, class: jclass, name: &str, signature: &str) -> Option<jmethodID> {
        let method_name = CString::new(name).unwrap();
        let method_signature = CString::new(signature).unwrap();
        let method_id = unsafe { (self.functions().GetMethodID)(*self, class, method_name.as_ptr(), method_signature.as_ptr()) };
        
        if method_id.is_null() {
            println!("Warning: failed to find method named {} : {}", name, signature);
            None
        } else {
            Some(method_id)
        }
    }
    
    fn find_field(&self, class: jclass, name: &str, signature: &str) -> Option<jfieldID>{
        let field_name = CString::new(name).unwrap();
        let field_signature = CString::new(signature).unwrap();
        let field_id = unsafe { (self.functions().GetFieldID)(*self, class, field_name.as_ptr(), field_signature.as_ptr()) };
        
        if field_id.is_null() {
            println!("Warning: failed to find field named {} : {}", name, signature);
            None
        } else {
            Some(field_id)
        }
    }
    
    fn find_static_method(&self, class: jclass, name: &str, signature: &str) -> Option<jmethodID> {
        let method_name = CString::new(name).unwrap();
        let method_signature = CString::new(signature).unwrap();
        let method_id = unsafe { (self.functions().GetStaticMethodID)(*self, class, method_name.as_ptr(), method_signature.as_ptr()) };
        
        if method_id.is_null() {
            println!("Warning: failed to find static method named {} : {}", name, signature);
            None
        } else {
            Some(method_id)
        }
    }
    
    fn find_static_field(&self, class: jclass, name: &str, signature: &str) -> Option<jfieldID>{
        let field_name = CString::new(name).unwrap();
        let field_signature = CString::new(signature).unwrap();
        let field_id = unsafe { (self.functions().GetStaticFieldID)(*self, class, field_name.as_ptr(), field_signature.as_ptr()) };
        
        if field_id.is_null() {
            println!("Warning: failed to find static field named {} : {}", name, signature);
            None
        } else {
            Some(field_id)
        }
    }
    
    fn delete_local_ref(&self, obj: jobject) {
        unsafe { (self.functions().DeleteLocalRef)(*self, obj); }
    }
    
    fn delete_global_ref(&self, obj: jobject) {
        unsafe { (self.functions().DeleteGlobalRef)(*self, obj); }
    }
    
    fn new_global_ref(&self, obj: jobject) -> Option<jobject> { 
        let global_obj_ref = unsafe { (self.functions().NewGlobalRef)(*self, obj) };
        
        if global_obj_ref.is_null() {
            None
        } else {
            Some(global_obj_ref)
        }
    }
    
    fn get_object_class(&self, obj: jobject) -> jclass {
        unsafe { (self.functions().GetObjectClass)(*self, obj) }
    }
    
    fn instance_of(&self, obj: jobject, class: jclass) -> bool {
        unsafe { (self.functions().IsInstanceOf)(*self, obj, class) != 0 }
    }
    
    fn has_exception(&self) -> bool {
        unsafe { (self.functions().ExceptionCheck)(*self) != 0 }
    }
    
    fn clear_exception(&self) {
        unsafe { (self.functions().ExceptionClear)(*self); }
    }
    
    fn describe_exception(&self) {
        unsafe { (self.functions().ExceptionDescribe)(*self); }
    }
    
    fn ensure_local_capacity(&self, size: i32) -> Option<()> {
        let result = unsafe { (self.functions().EnsureLocalCapacity)(*self, size) };
        if self.has_exception() {
            println!("Warning: exception raised during EnsureLocalCapacity");
            self.describe_exception();
            self.clear_exception();
            return None;
        }
        
        if result == 0 {
            Some(())
        } else {
            None
        }
    }
    
    fn functions<'a>(&'a self) -> &'a JNINativeInterface {
        unsafe { &(*(**self).functions) }
    }
}
