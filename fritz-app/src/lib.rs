#![warn(
    clippy::copy_iterator,
    clippy::default_trait_access,
    clippy::doc_link_with_quotes,
    clippy::enum_glob_use,
    clippy::expl_impl_clone_on_copy,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::items_after_statements,
    clippy::iter_not_returning_iterator,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::manual_instant_elapsed,
    clippy::manual_let_else,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::redundant_else,
    clippy::semicolon_if_nothing_returned,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_join,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unused_async,
    clippy::used_underscore_binding
)]
#![warn(clippy::wildcard_dependencies)]
#![warn(
    clippy::branches_sharing_code,
    clippy::clear_with_drain,
    clippy::cognitive_complexity,
    clippy::collection_is_never_read,
    clippy::debug_assert_with_mut_call,
    clippy::derive_partial_eq_without_eq,
    clippy::empty_line_after_doc_comments,
    clippy::empty_line_after_outer_attr,
    clippy::equatable_if_let,
    clippy::fallible_impl_from,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::iter_with_drain,
    clippy::large_stack_frames,
    clippy::manual_clamp,
    clippy::missing_const_for_fn,
    clippy::mutex_integer,
    clippy::needless_collect,
    clippy::nonstandard_macro_braces,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::redundant_clone,
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::suspicious_operation_groupings,
    clippy::trait_duplication_in_bounds,
    clippy::type_repetition_in_bounds,
    clippy::unnecessary_struct_initialization,
    clippy::unused_rounding,
    clippy::useless_let_if_seq
)]

pub mod api;
pub mod db;
pub mod fritz;
pub mod log;
pub mod ping;

#[cfg(test)]
mod test;
