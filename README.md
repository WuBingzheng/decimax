Fast, fixed-precision and floating-point decimal types.

It represents decimal fractions accurately by scaling integers in base-10.
So there is no round-off error like 0.1 + 0.2 != 0.3.

It is fixed-precision (the word "precision" here means significant digits).
It uses a single integer (`u32`, `u64`, or `u128`) as the underlying
representation, without involving heap memory allocation. This is fast and
`Copy`. However, arithmetic operations may lead to precision loss or
overflow. If this is undesirable, consider choosing an arbitrary-precision
decimal crate like `bigdecimal`.

It is floating-point. Each instance has its own scale, which changes during
arithmetic operations. It can represent a wider range, and users don’t need
to concern for scale, making it convenient to use. However, the implicit
rescaling introduces some performance overhead and 
[round-off errors](https://en.wikipedia.org/wiki/Floating-point_arithmetic#Addition_and_subtraction).
If this is undesirable, consider choosing an fixed-point decimal
crate like `primitive_fixed_point_decimal`.

Therefore, this crate is similar in kind to [`rust_decimal`](https://docs.rs/rust_decimal),
but better.


# Compare with `rust_decimal`

This crate has some advantages:

- Faster. For most cases of `+`, `-` and `*` operations, this crate is
1X ~ 10X faster than `rust_decimal`. While the `/` is more nuanced,
with both faster and slower cases. A typical comparison is shown below.
See the benchmark for details.

- More significant digits and scale. The 128-bit decimal type in this crate
has 121 bits for mantissa (about 36 decimal digits in base-10), while `rust_decimal`
has only 96 bits (about 28 decimal digits). Accordingly, our scale range is [0, 36],
compared to their [0, 28].

- More types. This crate provides 3 types: 128-bit, 64-bit, and 32-bit.


# How is it made faster?

In fact, there is no black magic in this crate. (Except for a fast division
algorithm which is used in just a few cases). I suspect the performance gain
isn’t so much because this crate is fast, but because `rust_decimal` is slow.

In this crate, the Decimal is defined as a single integer. Take the 128-bit type
as example:

```text
+-+-----+---------------------------+
|S|scale|  mantissa                 |
+-+-----+---------------------------+
```

The sign(`S`), scale, and mantissa occupy 1, 6, and 121 bits respectively.
Before each operation, they are unpacked via bitwise operations, and the
mantissa is still computed as one single `u128` value.

In contrast, the definition in `rust_decimal` is as follows:

```text
+--------+--------+--------+--------+
| flags  | high   | mid    | low    |
+--------+--------+--------+--------+
```

The mantissa consists of three `u32` components, and each operation requires
processing these three `u32` values in turn. Additionally, `rust_decimal` is
heavily optimized for small numbers. During computations, it handles cases
with 1, 2, and 3 `u32` segments separately. These conditional checks themselves,
along with the complex logic, may slow down the arithmetic operations.

You’ll get my point as long as you take a quick look at the code implementing
[the addition of this crate](xxx)
and [rust_decimal](https://docs.rs/crate/rust_decimal/latest/source/src/ops/add.rs).

In [`rust_decimal`'s document](https://docs.rs/rust_decimal/latest/rust_decimal/#comparison-to-other-decimal-implementations),
it's said that:

>  This structure allows us to make use of algorithmic optimizations to implement
>  basic arithmetic; ultimately this gives us the ability to squeeze out performance
>  and make it one of the fastest implementations available.

I don't understand this sentence. I have to guess that it was developed before
Rust's `u128` type was stabilized, when only `u64` or `u32` could be used for
calculations.

I’m not a performance expert, so the above is just my speculation. However,
the benchmark results are objective. Please check it out and run it yourself
if you are interested.

# Usage
