// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! This module provides a X.509 certificate parser.

#![allow(clippy::unused_unit)]
#![allow(clippy::needless_lifetimes)]

use std::convert::TryFrom;
use std::prelude::v1::*;

use yasna::models::TaggedDerValue;
use yasna::ASN1Error;
use yasna::{ASN1ErrorKind, ASN1Result};
use yasna::{BERReader, BERReaderSeq, BERReaderSet};
use yasna::{DERWriter, DERWriterSeq, DERWriterSet};

pub type Writer<'a> = DERWriter<'a>;
pub type Reader<'a, 'b> = BERReader<'a, 'b>;

pub trait ConsWriter {
    fn next(&mut self) -> Writer<'_>;
}

pub trait ConsReader<'a> {
    fn next(&mut self, tags: &[yasna::Tag]) -> ASN1Result<Reader<'a, '_>>;
}

impl ConsWriter for DERWriterSeq<'_> {
    fn next(&mut self) -> Writer<'_> {
        self.next()
    }
}

impl ConsWriter for DERWriterSet<'_> {
    fn next(&mut self) -> Writer<'_> {
        self.next()
    }
}

impl<'a> ConsReader<'a> for BERReaderSeq<'a, '_> {
    fn next(&mut self, _tags: &[yasna::Tag]) -> ASN1Result<Reader<'a, '_>> {
        Ok(self.next())
    }
}

impl<'a> ConsReader<'a> for BERReaderSet<'a, '_> {
    fn next(&mut self, tags: &[yasna::Tag]) -> ASN1Result<Reader<'a, '_>> {
        self.next(tags)
    }
}

pub trait Asn1Ty {
    type ValueTy;
    const TAG: yasna::Tag;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) -> ();
    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy>;
}

pub trait Asn1ConsTy
where
    Self: std::marker::Sized,
{
    type ValueTy;

    fn dump<W: ConsWriter>(writer: &mut W, value: Self::ValueTy) -> ();
    fn load<'a, R>(reader: &mut R) -> ASN1Result<Self::ValueTy>
    where
        R: ConsReader<'a>;
}

pub trait Asn1Tag {
    const TAG: yasna::Tag;
}

const fn context_tag(tnum: u64) -> yasna::Tag {
    yasna::Tag {
        tag_class: yasna::TagClass::ContextSpecific,
        tag_number: tnum,
    }
}

mod no_instance {
    #![allow(dead_code)]

    use super::{Asn1ConsTy, Asn1Tag, Asn1Ty};
    use std::marker::PhantomData;

    pub(crate) struct CtxT0;
    pub(crate) struct CtxT1;
    pub(crate) struct CtxT3;

    pub(crate) struct Boolean;
    pub(crate) struct Oid;
    pub(crate) struct U8;
    pub(crate) struct I8;
    pub(crate) struct BigUint;
    pub(crate) struct Utf8Str;
    pub(crate) struct UtcTime;
    pub(crate) struct BitVec;
    pub(crate) struct Bytes;
    pub(crate) struct DnsNameEntry;
    pub(crate) struct AlterNamesPayloadSeq;
    pub(crate) struct Tagged<T: Asn1Tag, S: Asn1Ty> {
        t: PhantomData<T>,
        s: PhantomData<S>,
    }
    pub(crate) struct Sequence<U: Asn1Ty, V: Asn1ConsTy> {
        u: PhantomData<U>,
        v: PhantomData<V>,
    }
    pub(crate) struct Set<U: Asn1Ty, V: Asn1ConsTy> {
        u: PhantomData<U>,
        v: PhantomData<V>,
    }
    pub(crate) struct Cons<U: Asn1Ty, V: Asn1ConsTy> {
        u: PhantomData<U>,
        v: PhantomData<V>,
    }
    pub(crate) struct Nil;
}

use no_instance::*;

impl Asn1Tag for CtxT0 {
    const TAG: yasna::Tag = context_tag(0);
}

impl Asn1Tag for CtxT1 {
    const TAG: yasna::Tag = context_tag(1);
}

impl Asn1Tag for CtxT3 {
    const TAG: yasna::Tag = context_tag(3);
}

impl<U: Asn1Ty, V: Asn1ConsTy> Asn1ConsTy for Cons<U, V> {
    type ValueTy = (U::ValueTy, V::ValueTy);

    fn dump<W: ConsWriter>(writer: &mut W, value: Self::ValueTy) -> () {
        U::dump(writer.next(), value.0);
        V::dump(writer, value.1);
    }

    fn load<'a, R>(reader: &mut R) -> ASN1Result<Self::ValueTy>
    where
        R: ConsReader<'a>,
    {
        let first = U::load(reader.next(&[U::TAG])?)?;
        let second = V::load(reader)?;
        Ok((first, second))
    }
}

impl Asn1ConsTy for Nil {
    type ValueTy = ();

    fn dump<W: ConsWriter>(_writer: &mut W, _value: Self::ValueTy) -> () {}
    fn load<'a, R>(_reader: &mut R) -> ASN1Result<Self::ValueTy>
    where
        R: ConsReader<'a>,
    {
        Ok(())
    }
}

impl<T: Asn1Tag, S: Asn1Ty> Asn1Ty for Tagged<T, S> {
    type ValueTy = S::ValueTy;
    const TAG: yasna::Tag = T::TAG;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_tagged(T::TAG, |writer| S::dump(writer, value));
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_tagged(T::TAG, S::load)
    }
}

impl<U: Asn1Ty, V: Asn1ConsTy> Asn1Ty for Sequence<U, V> {
    type ValueTy = (U::ValueTy, V::ValueTy);
    const TAG: yasna::Tag = yasna::tags::TAG_SEQUENCE;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_sequence(|writer| {
            U::dump(writer.next(), value.0);
            V::dump(writer, value.1);
        });
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_sequence(|reader| {
            let first = U::load(reader.next())?;
            let second = V::load(reader)?;
            Ok((first, second))
        })
    }
}

impl<U: Asn1Ty, V: Asn1ConsTy> Asn1Ty for Set<U, V> {
    type ValueTy = (U::ValueTy, V::ValueTy);
    const TAG: yasna::Tag = yasna::tags::TAG_SET;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_set(|writer| {
            U::dump(writer.next(), value.0);
            V::dump(writer, value.1);
        });
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_set(|reader| {
            let first = U::load(reader.next(&[U::TAG])?)?;
            let second = V::load(reader)?;
            Ok((first, second))
        })
    }
}

impl Asn1Ty for U8 {
    type ValueTy = u8;
    const TAG: yasna::Tag = yasna::tags::TAG_INTEGER;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_u8(value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_u8()
    }
}

impl Asn1Ty for I8 {
    type ValueTy = i8;
    const TAG: yasna::Tag = yasna::tags::TAG_INTEGER;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_i8(value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_i8()
    }
}

impl Asn1Ty for Boolean {
    type ValueTy = bool;
    const TAG: yasna::Tag = yasna::tags::TAG_BOOLEAN;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_bool(value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_bool()
    }
}

impl Asn1Ty for BigUint {
    type ValueTy = num_bigint::BigUint;
    const TAG: yasna::Tag = yasna::tags::TAG_INTEGER;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_biguint(&value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_biguint()
    }
}

impl Asn1Ty for Utf8Str {
    type ValueTy = String;
    const TAG: yasna::Tag = yasna::tags::TAG_UTF8STRING;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_utf8_string(value.as_str());
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_utf8string()
    }
}

impl Asn1Ty for Oid {
    type ValueTy = yasna::models::ObjectIdentifier;
    const TAG: yasna::Tag = yasna::tags::TAG_OID;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_oid(&value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_oid()
    }
}

impl Asn1Ty for UtcTime {
    type ValueTy = yasna::models::UTCTime;
    const TAG: yasna::Tag = yasna::tags::TAG_UTCTIME;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_utctime(&value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_utctime()
    }
}

impl Asn1Ty for BitVec {
    type ValueTy = bit_vec::BitVec;
    const TAG: yasna::Tag = yasna::tags::TAG_BITSTRING;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_bitvec(&value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_bitvec()
    }
}

impl Asn1Ty for Bytes {
    type ValueTy = Vec<u8>;
    const TAG: yasna::Tag = yasna::tags::TAG_OCTETSTRING;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_bytes(value.as_slice());
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        reader.read_bytes()
    }
}

impl Asn1Ty for AlterNamesPayloadSeq {
    type ValueTy = AlterNamesPayload;
    const TAG: yasna::Tag = yasna::tags::TAG_OCTETSTRING;

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        writer.write_bytes(value.as_slice());
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        let bytes = reader.read_bytes()?;
        AlterNamesPayload::try_from(bytes.as_slice())
    }
}

pub struct DnsName(String);

impl DnsName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl Asn1Ty for DnsNameEntry {
    type ValueTy = DnsName;
    const TAG: yasna::Tag = context_tag(2);

    fn dump(writer: Writer<'_>, value: Self::ValueTy) {
        let der_value = TaggedDerValue::from_tag_and_bytes(Self::TAG, value.into_vec());
        writer.write_tagged_der(&der_value);
    }

    fn load<'a>(reader: Reader<'a, '_>) -> ASN1Result<Self::ValueTy> {
        let der_value = reader.read_tagged_der()?;
        let bytes = der_value.value().to_vec();
        let name = String::from_utf8(bytes).map_err(|_| ASN1Error::new(ASN1ErrorKind::Invalid))?;
        Ok(DnsName::new(name))
    }
}

macro_rules! cons {
    () => { Nil };
    ($t: ty) => { Cons<$t, Nil> };
    ($t: ty, $($tt: ty),* $(,)?) => {
        Cons<$t, cons! { $($tt),* }>
    };
}

macro_rules! asn1_seq_ty {
    ($t: ty) => { Sequence<$t, Nil> };
    ($t: ty, $($tt: ty),* $(,)?) => {
        Sequence<$t, cons! { $($tt),* }>
    };
}

macro_rules! asn1_set_ty {
    ($t: ty) => { Set<$t, Nil> };
    ($t: ty, $($tt: ty),* $(,)?) => {
        Set<$t, cons! { $($tt),* }>
    };
}

macro_rules! asn1_seq {
    () => { () };
    ($e: expr) => {
        ($e, ())
    };
    ($e: expr , $($ee: expr),* $(,)?) => {
        ($e, asn1_seq!{ $($ee),* } )
    };
}

pub(crate) type Version = Tagged<CtxT0, I8>;
pub(crate) type Serial = U8;
pub(crate) type CertSignAlgo = asn1_seq_ty!(Oid);
pub(crate) type ValidRange = asn1_seq_ty!(UtcTime, UtcTime);
pub(crate) type Issuer = asn1_seq_ty!(asn1_set_ty!(asn1_seq_ty!(Oid, Utf8Str)));
pub(crate) type Subject = Issuer;
pub(crate) type PubKeyAlgo = asn1_seq_ty!(Oid, Oid);
pub(crate) type PubKey = asn1_seq_ty!(PubKeyAlgo, BitVec);
pub(crate) type RaCertExt = asn1_seq_ty!(Oid, Bytes);
pub(crate) type AlterSubjectNames = asn1_seq_ty!(DnsNameEntry, DnsNameEntry);

pub(crate) type AlterNamesExt = asn1_seq_ty!(Oid, AlterNamesPayloadSeq);
pub(crate) type CertExtensions = Tagged<CtxT3, asn1_seq_ty!(RaCertExt, AlterNamesExt)>;
pub(crate) type TbsCert = asn1_seq_ty!(
    Version,
    Serial,
    CertSignAlgo,
    Issuer,
    ValidRange,
    Subject,
    PubKey,
    CertExtensions,
);
pub(crate) type CertSig = BitVec;
pub(crate) type X509 = asn1_seq_ty!(TbsCert, CertSignAlgo, CertSig);

pub(crate) type BasicConstrainExt = asn1_seq_ty!(Oid, Boolean, Bytes);
pub(crate) type IntermediateCertExtensions = Tagged<CtxT3, asn1_seq_ty!(BasicConstrainExt)>;
pub(crate) type IntermediateCert = asn1_seq_ty!(
    Version,
    Serial,
    CertSignAlgo,
    Issuer,
    ValidRange,
    Subject,
    PubKey,
    IntermediateCertExtensions,
);
pub(crate) type _IntermediateX509 = asn1_seq_ty!(IntermediateCert, CertSignAlgo, CertSig);

pub(crate) struct AlterNamesPayload(Vec<u8>);
impl AlterNamesPayload {
    pub fn new(name1: DnsName, name2: DnsName) -> Self {
        let payload =
            yasna::construct_der(|writer| AlterSubjectNames::dump(writer, asn1_seq!(name1, name2)));
        AlterNamesPayload(payload)
    }
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<&[u8]> for AlterNamesPayload {
    type Error = ASN1Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let payload = yasna::parse_der(bytes, AlterSubjectNames::load)?;
        Ok(AlterNamesPayload::new(payload.0, payload.1 .0))
    }
}
