Fast, fixed-precision, floating-point decimal types.

It represents decimal fractions accurately by scaling integers in base-10.
So there is no round-off error like 0.1 + 0.2 != 0.3.

It is fixed-precision (the word "precision" here means significant digits).
It uses a single integer (`u128`, `u64`, or `u32`) as the underlying
representation, without involving heap memory allocation. This is fast and
`Copy`. However, arithmetic operations may lead to precision loss or
overflow. If this is undesirable, consider choosing an arbitrary-precision
decimal crate like `bigdecimal`.

It is floating-point. Each instance has its own scale, which changes during
arithmetic operations. It can represent a wider range, and is convenient to
use because users don’t need to concern for scale. However, the implicit
rescaling introduces performance overhead and
[round-off errors](https://en.wikipedia.org/wiki/Floating-point_arithmetic#Addition_and_subtraction).
If this is undesirable, consider choosing an fixed-point decimal
crate like `primitive_fixed_point_decimal`.

This crate is similar in kind to [`rust_decimal`](https://docs.rs/rust_decimal),
but *better*.


# Compare with `rust_decimal`

This crate has these advantages:

- Much faster. This crate is 2X ~ 6X faster than `rust_decimal` at `+`, `-`
and `*` operations. While the `/` is complex, with both faster and slower
cases. A typical comparison is shown in below chart. See
the [benchmark](https://github.com/WuBingzheng/lean-decimal/blob/v0.1.0/benches/README.md)
for details.

![Benchmark result](https://raw.githubusercontent.com/WuBingzheng/lean-decimal/refs/heads/v0.1.0/benches/charts/mul-amd.svg)

- More significant digits and scale. The 128-bit decimal type in this crate
has 121 bits for mantissa (about 36 decimal digits in base-10), while `rust_decimal`
has only 96 bits (about 28 decimal digits). Accordingly, our scale range is
[0, 36], compared to their [0, 28].

- More types. This crate provides 3 types: 128-bit, 64-bit, and 32-bit. The
last two are in process, and will be available in next version.


# How is it made faster?

In fact, there is no black magic in this crate (except for a fast division
algorithm which is used in just a few cases). I suspect the performance gain
isn’t so much because this crate is fast, but because `rust_decimal` is slow.

In this crate, the decimal is defined as a single integer. Take the 128-bit type
as example:

```text
+-+-----+-------------------------------+
|S|scale|  mantissa                     |
+-+-----+-------------------------------+
```

The sign(`S`), scale, and mantissa occupy 1, 6, and 121 bits respectively.
Before each operation, they are unpacked via bitwise operations, and the
mantissa is still computed as one single `u128` value.

In contrast, the definition in `rust_decimal` is as follows:

```text
+---------+---------+---------+---------+
| flags   | high    | mid     | low     |
+---------+---------+---------+---------+
```

The mantissa consists of three `u32` components, and each operation requires
processing these three `u32` values in turn. Additionally, `rust_decimal`
checks whether the two operands are small(32 bit), medium(64 bit), or
large(96 bit), and uses different computation processes accordingly.
These conditional checks themselves, along with the complex logic, may
slow down the arithmetic operations.

You’ll get my point as long as you take a quick look at the code of the
addition implementation
[of this crate](https://docs.rs/crate/lean-decimal/0.1.0/source/src/lib.rs#32)
and [of rust_decimal](https://docs.rs/crate/rust_decimal/1.41.0/source/src/ops/add.rs).

In [`rust_decimal`'s documentation](https://docs.rs/rust_decimal/1.41.0/rust_decimal/#comparison-to-other-decimal-implementations),
it's said that:

>  This structure allows us to make use of algorithmic optimizations to implement
>  basic arithmetic; ultimately this gives us the ability to squeeze out performance
>  and make it one of the fastest implementations available.

I don't quite understand this sentence. I have to guess that it was developed
before Rust's `u128` type was stabilized, when only `u64` or `u32` could be used.

I’m not a performance expert, so the above is just my speculation. However,
the [benchmark results](https://github.com/WuBingzheng/lean-decimal/blob/v0.1.0/benches/README.md)
are objective. Please check it out and run it yourself.


# Usage

```rust
// We take the 128-bit type as example.
use lean_decimal::Dec128;
use core::str::FromStr;

// Construct from integer and string, while the float is in process.
let a = Dec128::from(123);
let b = Dec128::from_str("123.456").unwrap();

// Construct from mantissa and scale.
let b2 = Dec128::from_parts(123456, 3);
assert_eq!(b, b2);

// Addition and substraction operate with same type only.
assert_eq!(a + b, Dec128::from_parts(246456, 3)); // 123 + 123.456 = 246.456

// Multiplication and division can operate with short integers and decimals too.
assert_eq!(b * 2, Dec128::from_parts(246912, 3)); // 123.456 * 2 = 246.912
```
