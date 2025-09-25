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

use anyhow::{anyhow, Result};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

pub fn multiplication_f64_u128(a: f64, b: u128) -> Result<u128> {
    let a =
        Decimal::from_f64(a).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let b = Decimal::from_u128(b)
        .ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", b))?;
    let c = a
        .checked_mul(b)
        .ok_or_else(|| anyhow!("Failed to multiply {} and {}", a, b))?;
    let c = c
        .to_u128()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to u128", c))?;
    Ok(c)
}

pub fn multiplication_f64_u128_to_f64(a: f64, b: u128) -> Result<f64> {
    let a =
        Decimal::from_f64(a).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let b = Decimal::from_u128(b)
        .ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", b))?;
    let c = a
        .checked_mul(b)
        .ok_or_else(|| anyhow!("Failed to multiply {} and {}", a, b))?;
    let c = c.round_dp(8);
    let c = c
        .to_f64()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to f64", c))?;
    Ok(c)
}

pub fn round_f64(a: f64) -> Result<f64> {
    let a =
        Decimal::from_f64(a).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let a = a.round_dp(8);
    let a = a
        .to_f64()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to f64", a))?;
    Ok(a)
}

pub fn division_u128_u128(a: u128, b: u128) -> Result<f64> {
    let a = Decimal::from_u128(a)
        .ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let b = Decimal::from_u128(b)
        .ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", b))?;
    let c = a
        .checked_div(b)
        .ok_or_else(|| anyhow!("Failed to divide {} and {}", a, b))?;
    let c = c.round_dp(18); // the minimum value is 1 wei for Ethereum
    let c = c
        .to_f64()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to f64", c))?;
    Ok(c)
}

pub fn multiplication_f64_f64(a: f64, b: f64) -> Result<f64> {
    let a =
        Decimal::from_f64(a).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let b =
        Decimal::from_f64(b).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", b))?;
    let c = a
        .checked_mul(b)
        .ok_or_else(|| anyhow!("Failed to multiply {} and {}", a, b))?;
    let c = c.round_dp(8);
    let c = c
        .to_f64()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to f64", c))?;
    Ok(c)
}

pub fn division_f64_f64(a: f64, b: f64) -> Result<f64> {
    let a =
        Decimal::from_f64(a).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", a))?;
    let b =
        Decimal::from_f64(b).ok_or_else(|| anyhow!("Failed to convert amount {} to decimal", b))?;
    let c = a
        .checked_div(b)
        .ok_or_else(|| anyhow!("Failed to divide {} and {}", a, b))?;
    let c = c.round_dp(8);
    let c = c
        .to_f64()
        .ok_or_else(|| anyhow!("Failed to convert amount {} to f64", c))?;
    Ok(c)
}

pub fn u128_to_f64(value: u128) -> Result<f64> {
    // Check if u128 value can be safely represented as f64
    // f64 can represent integers up to 2^53 exactly (about 9.0e15)
    const MAX_SAFE_F64_INT: u128 = (1u128 << 53) - 1;

    if value > MAX_SAFE_F64_INT {
        return Err(anyhow!(
            "Value {} exceeds safe f64 conversion limit {}",
            value,
            MAX_SAFE_F64_INT
        ));
    }

    Ok(value as f64)
}
