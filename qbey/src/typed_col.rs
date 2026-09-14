//! Typed column references.
//!
//! [`TypedCol<T>`] is a [`Col`] that remembers the Rust type of the values it
//! holds. It is produced by [`qbey_schema!`](crate::qbey_schema) when a column is
//! declared as `name: Type`, and it narrows the comparison methods so that
//! passing an unrelated type is a compile error rather than a silent bug.
//!
//! The bind value itself is still converted by the query's value type, so a
//! newtype ID only needs `impl From<MyId> for Value` (or for whatever custom
//! value type the query uses) to flow all the way through to the driver.

use std::marker::PhantomData;

use crate::column::{Col, ColCondition, SelectItem};
use crate::like::LikeExpression;
use crate::value::Op;
use crate::where_clause::{IntoRangeClause, WhereClause};

/// A [`Col`] carrying the Rust type of the values stored in that column.
///
/// See the [module documentation](self) for the rationale, and
/// [`qbey_schema!`](crate::qbey_schema) for how to declare typed columns.
///
/// # Rejecting a value of the wrong type
///
/// ```compile_fail
/// use qbey::{qbey, qbey_schema, Value};
/// use qbey::prelude::*;
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Users, "users", [id: UserId, name: String]);
///
/// let u = Users::new();
/// let mut q = qbey(&u);
/// // `id` is a `UserId` column, so a string is not accepted.
/// q.and_where(u.id().eq("foo"));
/// ```
///
/// The same query compiles once the right type is used:
///
/// ```
/// use qbey::{qbey, qbey_schema, Value};
/// use qbey::prelude::*;
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Users, "users", [id: UserId, name: String]);
///
/// let u = Users::new();
/// let mut q = qbey(&u);
/// q.and_where(u.id().eq(UserId(1)));
/// let (sql, _binds) = q.to_sql();
/// assert_eq!(sql, r#"SELECT * FROM "users" WHERE "users"."id" = ?"#);
/// ```
///
/// # Rejecting a different ID type
///
/// ```compile_fail
/// use qbey::{qbey, qbey_schema, Value};
/// use qbey::prelude::*;
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// #[derive(Debug, Clone)]
/// struct PostId(i64);
/// impl From<UserId> for Value {
///     fn from(id: UserId) -> Self { Value::Int(id.0) }
/// }
///
/// qbey_schema!(Users, "users", [id: UserId]);
///
/// let u = Users::new();
/// let mut q = qbey(&u);
/// // `PostId` is a different type, even though both wrap `i64`.
/// q.and_where(u.id().eq(PostId(1)));
/// ```
///
/// # Rejecting a join between mismatched ID types
///
/// ```compile_fail
/// use qbey::{qbey, qbey_schema, Value};
/// use qbey::prelude::*;
///
/// #[derive(Debug, Clone)]
/// struct UserId(i64);
/// #[derive(Debug, Clone)]
/// struct PostId(i64);
///
/// qbey_schema!(Users, "users", [id: UserId]);
/// qbey_schema!(Posts, "posts", [id: PostId, author_id: UserId]);
///
/// let u = Users::new();
/// let p = Posts::new();
/// let mut q = qbey(&u);
/// // `posts.id` is a `PostId`, so it cannot be joined to `users.id`.
/// q.join(&p, u.id().eq(p.id()));
/// ```
pub struct TypedCol<T> {
    col: Col,
    // `fn() -> T` keeps `TypedCol<T>` covariant in `T` without requiring `T` to
    // be `Send`/`Sync`, and keeps auto-trait impls independent of `T`.
    _ty: PhantomData<fn() -> T>,
}

// Implemented by hand: `derive` would add a `T: Clone` / `T: Debug` bound, but
// `T` is only a marker here and `select(&[...])` requires `Clone` regardless.
impl<T> Clone for TypedCol<T> {
    fn clone(&self) -> Self {
        TypedCol {
            col: self.col.clone(),
            _ty: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for TypedCol<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TypedCol").field(&self.col).finish()
    }
}

impl<T> TypedCol<T> {
    /// Wrap a [`Col`] with a value type. Used by `qbey_schema!`.
    #[doc(hidden)]
    pub fn new(col: Col) -> Self {
        TypedCol {
            col,
            _ty: PhantomData,
        }
    }

    /// Discard the type information, yielding the underlying [`Col`].
    ///
    /// Use this to mix columns of different types in a single `select(&[...])`
    /// slice, or to reach a `Col` method that has no typed equivalent.
    pub fn into_col(self) -> Col {
        self.col
    }

    /// Borrow the underlying [`Col`].
    pub fn as_col(&self) -> &Col {
        &self.col
    }

    /// Alias this column in a SELECT list, keeping the value type.
    pub fn as_(self, alias: &str) -> Self {
        TypedCol {
            col: self.col.as_(alias),
            _ty: PhantomData,
        }
    }

    /// Sort ascending by this column.
    pub fn asc(self) -> crate::column::OrderByClause {
        self.col.asc()
    }

    /// Sort descending by this column.
    pub fn desc(self) -> crate::column::OrderByClause {
        self.col.desc()
    }

    /// Equality (`=`), against a value of type `T` or another `TypedCol<T>`.
    pub fn eq<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Eq)
    }

    /// Inequality (`!=`).
    pub fn ne<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Ne)
    }

    /// Greater than (`>`).
    pub fn gt<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Gt)
    }

    /// Less than (`<`).
    pub fn lt<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Lt)
    }

    /// Greater than or equal (`>=`).
    pub fn gte<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Gte)
    }

    /// Less than or equal (`<=`).
    pub fn lte<R: TypedRhs<T>>(self, rhs: R) -> R::Output {
        rhs.apply_condition(self.col, Op::Lte)
    }
}

impl<T: Clone> TypedCol<T> {
    /// `IN (...)` over values of the column's own type.
    ///
    /// An empty slice produces `1 = 0`, matching [`ConditionExpr::included`].
    ///
    /// [`ConditionExpr::included`]: crate::ConditionExpr::included
    pub fn included(self, vals: &[T]) -> WhereClause<T> {
        WhereClause::In {
            col: self.col,
            vals: vals.to_vec(),
        }
    }

    /// `NOT IN (...)` over values of the column's own type.
    ///
    /// An empty slice produces `1 = 1`, matching [`ConditionExpr::not_included`].
    ///
    /// [`ConditionExpr::not_included`]: crate::ConditionExpr::not_included
    pub fn not_included(self, vals: &[T]) -> WhereClause<T> {
        WhereClause::NotIn {
            col: self.col,
            vals: vals.to_vec(),
        }
    }

    /// `BETWEEN low AND high`.
    pub fn between(self, low: T, high: T) -> WhereClause<T> {
        WhereClause::Between {
            col: self.col,
            low,
            high,
        }
    }

    /// `NOT BETWEEN low AND high`.
    pub fn not_between(self, low: T, high: T) -> WhereClause<T> {
        WhereClause::NotBetween {
            col: self.col,
            low,
            high,
        }
    }

    /// Convert a Rust range into SQL conditions, as
    /// [`ConditionExpr::in_range`](crate::ConditionExpr::in_range) does.
    pub fn in_range(self, range: impl IntoRangeClause<T>) -> WhereClause<T> {
        range.into_where_clause(self.col)
    }
}

/// `LIKE` is only offered on text columns — a `TypedCol<UserId>` has no
/// meaningful pattern match.
impl TypedCol<String> {
    /// `LIKE` with a safely-escaped [`LikeExpression`].
    pub fn like(self, expr: LikeExpression) -> WhereClause<String> {
        let val = expr.to_pattern();
        WhereClause::Like {
            col: self.col,
            expr,
            val,
        }
    }

    /// `NOT LIKE` with a safely-escaped [`LikeExpression`].
    pub fn not_like(self, expr: LikeExpression) -> WhereClause<String> {
        let val = expr.to_pattern();
        WhereClause::NotLike {
            col: self.col,
            expr,
            val,
        }
    }
}

impl<T> From<TypedCol<T>> for Col {
    fn from(t: TypedCol<T>) -> Col {
        t.col
    }
}

impl<T, V: Clone> From<TypedCol<T>> for SelectItem<V> {
    fn from(t: TypedCol<T>) -> SelectItem<V> {
        SelectItem::Col(t.col)
    }
}

/// The right-hand side of a comparison on a [`TypedCol<T>`].
///
/// This is the typed counterpart of [`ConditionRhs`](crate::column::ConditionRhs):
/// it is implemented for `T` itself, for `&T`, and for another `TypedCol<T>`
/// (column-to-column comparison), which is what makes a join between two
/// different ID types fail to compile.
pub trait TypedRhs<T> {
    /// [`WhereClause<T>`] for value comparisons, [`ColCondition`] for
    /// column-to-column comparisons.
    type Output;
    fn apply_condition(self, col: Col, op: Op) -> Self::Output;
}

impl<T: Clone> TypedRhs<T> for T {
    type Output = WhereClause<T>;
    fn apply_condition(self, col: Col, op: Op) -> WhereClause<T> {
        WhereClause::Condition { col, op, val: self }
    }
}

impl<T: Clone> TypedRhs<T> for &T {
    type Output = WhereClause<T>;
    fn apply_condition(self, col: Col, op: Op) -> WhereClause<T> {
        WhereClause::Condition {
            col,
            op,
            val: self.clone(),
        }
    }
}

impl<T> TypedRhs<T> for TypedCol<T> {
    type Output = ColCondition;
    fn apply_condition(self, left: Col, op: Op) -> ColCondition {
        ColCondition {
            left,
            op,
            right: self.col,
        }
    }
}

/// String columns accept string literals directly, so `eq("x")` works without
/// an explicit `.to_string()`.
impl TypedRhs<String> for &str {
    type Output = WhereClause<String>;
    fn apply_condition(self, col: Col, op: Op) -> WhereClause<String> {
        WhereClause::Condition {
            col,
            op,
            val: self.to_string(),
        }
    }
}
