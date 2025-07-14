// modified version of:
// https://github.com/tree-sitter/tree-sitter/blob/master/highlight/src/lib.rs
//
// main changes:
//   - remove all parsing (take in tree directly)
//   - removed injections
//   - support for ropey

use ropey::RopeSlice;
use std::{
    borrow::Cow,
    iter,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops, slice, str,
};
use tree_sitter::{
    ffi, Language, Node, Point, Query, QueryCapture, QueryCaptures, QueryCursor, QueryError,
    QueryMatch, Range, TextProvider, Tree,
};

/// Indicates which highlight should be applied to a region of source code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Highlight(pub usize);

/// Represents a single step in rendering a syntax-highlighted document.
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum HighlightEvent {
    Source { start: usize, end: usize },
    HighlightStart(Highlight),
    HighlightEnd,
}

/// Contains the data needed to highlight code written in a particular language.
///
/// This struct is immutable and can be shared between threads.
#[allow(dead_code)]
pub struct HighlightConfiguration {
    pub language: Language,
    pub language_name: String,
    pub query: Query,
    combined_injections_query: Option<Query>,
    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    highlight_indices: Vec<Option<Highlight>>,
    non_local_variable_patterns: Vec<bool>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,
}

/// Performs syntax highlighting, recognizing a given list of highlight names.
///
/// For the best performance `Highlighter` values should be reused between
/// syntax highlighting calls. A separate highlighter is needed for each thread that
/// is performing highlighting.
pub struct Highlighter {
    cursors: Vec<QueryCursor>,
}

#[derive(Debug)]
struct LocalDef<'a> {
    name: Cow<'a, str>,
    value_range: ops::Range<usize>,
    highlight: Option<Highlight>,
}

#[derive(Debug)]
struct LocalScope<'a> {
    inherits: bool,
    range: ops::Range<usize>,
    local_defs: Vec<LocalDef<'a>>,
}

struct HighlightIter<'a> {
    source: RopeProvider<'a>,
    byte_offset: usize,
    highlighter: &'a mut Highlighter,
    layers: Vec<HighlightIterLayer<'a>>,
    next_event: Option<HighlightEvent>,
    last_highlight_range: Option<(usize, usize, usize)>,
}

#[allow(dead_code)]
struct HighlightIterLayer<'a> {
    _tree: Option<Tree>,
    cursor: QueryCursor,
    captures: iter::Peekable<_QueryCaptures<'a, 'a, RopeProvider<'a>, &'a [u8]>>,
    config: &'a HighlightConfiguration,
    highlight_end_stack: Vec<usize>,
    scope_stack: Vec<LocalScope<'a>>,
    ranges: Vec<Range>,
    depth: usize,
}

pub struct _QueryCaptures<'query, 'tree: 'query, T: TextProvider<I>, I: AsRef<[u8]>> {
    ptr: *mut ffi::TSQueryCursor,
    query: &'query Query,
    text_provider: T,
    buffer1: Vec<u8>,
    buffer2: Vec<u8>,
    _current_match: Option<(QueryMatch<'query, 'tree>, usize)>,
    _options: Option<*mut ffi::TSQueryCursorOptions>,
    _phantom: PhantomData<(&'tree (), I)>,
}
struct _QueryMatch<'cursor, 'tree> {
    pub _pattern_index: usize,
    pub _captures: &'cursor [QueryCapture<'tree>],
    _id: u32,
    _cursor: *mut ffi::TSQueryCursor,
}
impl<'tree> _QueryMatch<'_, 'tree> {
    fn new(m: &ffi::TSQueryMatch, cursor: *mut ffi::TSQueryCursor) -> Self {
        _QueryMatch {
            _cursor: cursor,
            _id: m.id,
            _pattern_index: m.pattern_index as usize,
            _captures: if m.capture_count > 0 { unsafe {
                    slice::from_raw_parts(
                        m.captures.cast::<QueryCapture<'tree>>(),
                        m.capture_count as usize,
                    )
                } } else { Default::default() },
        }
    }
}

impl<'query, 'tree: 'query, T: TextProvider<I>, I: AsRef<[u8]>> Iterator
    for _QueryCaptures<'query, 'tree, T, I>
{
    type Item = (QueryMatch<'query, 'tree>, usize);
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                let mut capture_index = 0u32;
                let mut m = MaybeUninit::<ffi::TSQueryMatch>::uninit();
                if ffi::ts_query_cursor_next_capture(
                    self.ptr,
                    m.as_mut_ptr(),
                    core::ptr::addr_of_mut!(capture_index),
                ) {
                    let result = std::mem::transmute::<_QueryMatch, QueryMatch>(_QueryMatch::new(
                        &m.assume_init(),
                        self.ptr,
                    ));
                    if result.satisfies_text_predicates(
                        self.query,
                        &mut self.buffer1,
                        &mut self.buffer2,
                        &mut self.text_provider,
                    ) {
                        return Some((result, capture_index as usize));
                    }
                    result.remove();
                } else {
                    return None;
                }
            }
        }
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cursors: Vec::new(),
        }
    }

    /// Iterate over the highlighted regions for a given node.
    /// Does not parse anything (and therefore does not support injections).
    pub fn highlight_existing_tree<'a>(
        &'a mut self,
        source: RopeSlice<'a>,
        node: Node,
        config: &'a HighlightConfiguration,
    ) -> impl Iterator<Item = HighlightEvent> + 'a {
        let rope_provider = RopeProvider(source);
        let layers = vec![HighlightIterLayer::new_from_tree(
            rope_provider,
            node,
            config,
        )];
        let mut result = HighlightIter {
            source: rope_provider,
            byte_offset: 0,
            highlighter: self,
            layers,
            next_event: None,
            last_highlight_range: None,
        };
        result.sort_layers();
        result
    }
}

impl HighlightConfiguration {
    /// Creates a `HighlightConfiguration` for a given `Language` and set of highlighting
    /// queries.
    ///
    /// # Parameters
    ///
    /// * `language`  - The Tree-sitter `Language` that should be used for parsing.
    /// * `highlights_query` - A string containing tree patterns for syntax highlighting. This
    ///   should be non-empty, otherwise no syntax highlights will be added.
    /// * `injections_query` -  A string containing tree patterns for injecting other languages into
    ///   the document. This can be empty if no injections are desired.
    /// * `locals_query` - A string containing tree patterns for tracking local variable definitions
    ///   and references. This can be empty if local variable tracking is not needed.
    ///
    /// Returns a `HighlightConfiguration` that can then be used with the `highlight` method.
    pub fn new(
        language: Language,
        name: impl Into<String>,
        highlights_query: &str,
        injection_query: &str,
        locals_query: &str,
    ) -> Result<Self, QueryError> {
        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(injection_query);
        let locals_query_offset = query_source.len();
        query_source.push_str(locals_query);
        let highlights_query_offset = query_source.len();
        query_source.push_str(highlights_query);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let mut query = Query::new(&language, &query_source)?;
        let mut locals_pattern_index = 0;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                if pattern_offset < highlights_query_offset {
                    highlights_pattern_index += 1;
                }
                if pattern_offset < locals_query_offset {
                    locals_pattern_index += 1;
                }
            }
        }

        // Construct a separate query just for dealing with the 'combined injections'.
        // Disable the combined injection patterns in the main query.
        let mut combined_injections_query = Query::new(&language, injection_query)?;
        let mut has_combined_queries = false;
        for pattern_index in 0..locals_pattern_index {
            let settings = query.property_settings(pattern_index);
            if settings.iter().any(|s| &*s.key == "injection.combined") {
                has_combined_queries = true;
                query.disable_pattern(pattern_index);
            } else {
                combined_injections_query.disable_pattern(pattern_index);
            }
        }
        let combined_injections_query = if has_combined_queries {
            Some(combined_injections_query)
        } else {
            None
        };

        // Find all of the highlighting patterns that are disabled for nodes that
        // have been identified as local variables.
        let non_local_variable_patterns = (0..query.pattern_count())
            .map(|i| {
                query
                    .property_predicates(i)
                    .iter()
                    .any(|(prop, positive)| !*positive && prop.key.as_ref() == "local")
            })
            .collect();

        // Store the numeric ids for all of the special captures.
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match *name {
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        let highlight_indices = vec![None; query.capture_names().len()];
        Ok(Self {
            language,
            language_name: name.into(),
            query,
            combined_injections_query,
            locals_pattern_index,
            highlights_pattern_index,
            highlight_indices,
            non_local_variable_patterns,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
            local_scope_capture_index,
        })
    }

    /// Set the list of recognized highlight names.
    ///
    /// Tree-sitter syntax-highlighting queries specify highlights in the form of dot-separated
    /// highlight names like `punctuation.bracket` and `function.method.builtin`. Consumers of
    /// these queries can choose to recognize highlights with different levels of specificity.
    /// For example, the string `function.builtin` will match against `function.method.builtin`
    /// and `function.builtin.constructor`, but will not match `function.method`.
    ///
    /// When highlighting, results are returned as `Highlight` values, which contain the index
    /// of the matched highlight this list of highlight names.
    pub fn configure(&mut self, recognized_names: &[impl AsRef<str>]) {
        let mut capture_parts = Vec::new();
        self.highlight_indices.clear();
        self.highlight_indices
            .extend(self.query.capture_names().iter().map(move |capture_name| {
                capture_parts.clear();
                capture_parts.extend(capture_name.split('.'));

                let mut best_index = None;
                let mut best_match_len = 0;
                for (i, recognized_name) in recognized_names.iter().enumerate() {
                    let mut len = 0;
                    let mut matches = true;
                    for part in recognized_name.as_ref().split('.') {
                        len += 1;
                        if !capture_parts.contains(&part) {
                            matches = false;
                            break;
                        }
                    }
                    if matches && len > best_match_len {
                        best_index = Some(i);
                        best_match_len = len;
                    }
                }
                best_index.map(Highlight)
            }));
    }
}

impl<'a> HighlightIterLayer<'a> {
    fn new_from_tree(
        source: RopeProvider<'a>,
        node: Node,
        config: &'a HighlightConfiguration,
    ) -> Self {
        let mut cursor = QueryCursor::new();

        // `QueryCursor` is really just a pointer, so it's ok to move.
        let cursor_ref = unsafe {
            mem::transmute::<&mut tree_sitter::QueryCursor, &'static mut QueryCursor>(&mut cursor)
        };
        let captures =
            unsafe {
                std::mem::transmute::<QueryCaptures<_, _>, _QueryCaptures<_, _>>(
                    cursor_ref.captures(&config.query, node, source),
                )
            }
            .peekable();

        HighlightIterLayer {
            highlight_end_stack: Vec::new(),
            scope_stack: vec![LocalScope {
                inherits: false,
                range: 0..usize::MAX,
                local_defs: Vec::new(),
            }],
            cursor,
            depth: 0,
            _tree: None,
            captures,
            config,
            ranges: vec![Range {
                start_byte: 0,
                end_byte: usize::MAX,
                start_point: Point::new(0, 0),
                end_point: Point::new(usize::MAX, usize::MAX),
            }],
        }
    }

    // First, sort scope boundaries by their byte offset in the document. At a
    // given position, emit scope endings before scope beginnings. Finally, emit
    // scope boundaries from deeper layers first.
    fn sort_key(&mut self) -> Option<(usize, bool, isize)> {
        let depth = -(self.depth as isize);
        let next_start = self
            .captures
            .peek()
            .map(|(m, i)| m.captures[*i].node.start_byte());
        let next_end = self.highlight_end_stack.last().copied();
        match (next_start, next_end) {
            (Some(start), Some(end)) => {
                if start < end {
                    Some((start, true, depth))
                } else {
                    Some((end, false, depth))
                }
            }
            (Some(i), None) => Some((i, true, depth)),
            (None, Some(j)) => Some((j, false, depth)),
            _ => None,
        }
    }
}

impl HighlightIter<'_> {
    fn emit_event(
        &mut self,
        offset: usize,
        event: Option<HighlightEvent>,
    ) -> Option<HighlightEvent> {
        let result;
        if self.byte_offset < offset {
            result = Some(HighlightEvent::Source {
                start: self.byte_offset,
                end: offset,
            });
            self.byte_offset = offset;
            self.next_event = event;
        } else {
            result = event
        }
        self.sort_layers();
        result
    }

    fn sort_layers(&mut self) {
        while !self.layers.is_empty() {
            if let Some(sort_key) = self.layers[0].sort_key() {
                let mut i = 0;
                while i + 1 < self.layers.len() {
                    if let Some(next_offset) = self.layers[i + 1].sort_key() {
                        if next_offset < sort_key {
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                if i > 0 {
                    self.layers[0..=i].rotate_left(1);
                }
                break;
            }
            let layer = self.layers.remove(0);
            self.highlighter.cursors.push(layer.cursor);
        }
    }
}

impl Iterator for HighlightIter<'_> {
    type Item = HighlightEvent;

    #[allow(unused_assignments)]
    fn next(&mut self) -> Option<Self::Item> {
        'main: loop {
            // If we've already determined the next highlight boundary, just return it.
            if let Some(e) = self.next_event.take() {
                return Some(e);
            }

            // If none of the layers have any more highlight boundaries, terminate.
            if self.layers.is_empty() {
                return if self.byte_offset < self.source.0.len_bytes() {
                    let result = Some(HighlightEvent::Source {
                        start: self.byte_offset,
                        end: self.source.0.len_bytes(),
                    });
                    self.byte_offset = self.source.0.len_bytes();
                    result
                } else {
                    None
                };
            }

            // Get the next capture from whichever layer has the earliest highlight boundary.
            let range;
            let layer = &mut self.layers[0];
            if let Some((next_match, capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*capture_index];
                range = next_capture.node.byte_range();

                // If any previous highlight ends before this node starts, then before
                // processing this capture, emit the source code up until the end of the
                // previous highlight, and an end event for that highlight.
                if let Some(end_byte) = layer.highlight_end_stack.last().copied() {
                    if end_byte <= range.start {
                        layer.highlight_end_stack.pop();
                        return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
                    }
                }
            }
            // If there are no more captures, then emit any remaining highlight end events.
            // And if there are none of those, then just advance to the end of the document.
            else {
                if let Some(end_byte) = layer.highlight_end_stack.last().copied() {
                    layer.highlight_end_stack.pop();
                    return self.emit_event(end_byte, Some(HighlightEvent::HighlightEnd));
                }
                return self.emit_event(self.source.0.len_bytes(), None);
            }

            let (mut match_, capture_index) = layer.captures.next().unwrap();
            let mut capture = match_.captures[capture_index];

            // Remove from the local scope stack any local scopes that have already ended.
            while range.start > layer.scope_stack.last().unwrap().range.end {
                layer.scope_stack.pop();
            }

            // If this capture is for tracking local variables, then process the
            // local variable info.
            let mut reference_highlight = None;
            let mut definition_highlight = None;
            while match_.pattern_index < layer.config.highlights_pattern_index {
                // If the node represents a local scope, push a new local scope onto
                // the scope stack.
                if Some(capture.index) == layer.config.local_scope_capture_index {
                    definition_highlight = None;
                    let mut scope = LocalScope {
                        inherits: true,
                        range: range.clone(),
                        local_defs: Vec::new(),
                    };
                    for prop in layer.config.query.property_settings(match_.pattern_index) {
                        if prop.key.as_ref() == "local.scope-inherits" {
                            scope.inherits =
                                prop.value.as_ref().is_none_or(|r| r.as_ref() == "true");
                        }
                    }
                    layer.scope_stack.push(scope);
                }
                // If the node represents a definition, add a new definition to the
                // local scope at the top of the scope stack.
                else if Some(capture.index) == layer.config.local_def_capture_index {
                    reference_highlight = None;
                    definition_highlight = None;
                    let scope = layer.scope_stack.last_mut().unwrap();

                    let mut value_range = 0..0;
                    for capture in match_.captures {
                        if Some(capture.index) == layer.config.local_def_value_capture_index {
                            value_range = capture.node.byte_range();
                        }
                    }

                    let name = Cow::from(self.source.0.byte_slice(range.clone()));
                    scope.local_defs.push(LocalDef {
                        name,
                        value_range,
                        highlight: None,
                    });
                    definition_highlight = scope.local_defs.last_mut().map(|s| &mut s.highlight);
                }
                // If the node represents a reference, then try to find the corresponding
                // definition in the scope stack.
                else if Some(capture.index) == layer.config.local_ref_capture_index
                    && definition_highlight.is_none()
                {
                    definition_highlight = None;
                    let name = self.source.0.byte_slice(range.clone());
                    for scope in layer.scope_stack.iter().rev() {
                        if let Some(highlight) = scope.local_defs.iter().rev().find_map(|def| {
                            if def.name == name && range.start >= def.value_range.end {
                                Some(def.highlight)
                            } else {
                                None
                            }
                        }) {
                            reference_highlight = highlight;
                            break;
                        }
                        if !scope.inherits {
                            break;
                        }
                    }
                }

                // Continue processing any additional matches for the same node.
                if let Some((next_match, next_capture_index)) = layer.captures.peek() {
                    let next_capture = next_match.captures[*next_capture_index];
                    if next_capture.node == capture.node {
                        capture = next_capture;
                        match_ = layer.captures.next().unwrap().0;
                        continue;
                    }
                }

                self.sort_layers();
                continue 'main;
            }

            // Otherwise, this capture must represent a highlight.
            // If this exact range has already been highlighted by an earlier pattern, or by
            // a different layer, then skip over this one.
            if let Some((last_start, last_end, last_depth)) = self.last_highlight_range {
                if range.start == last_start && range.end == last_end && layer.depth < last_depth {
                    self.sort_layers();
                    continue 'main;
                }
            }

            // Once a highlighting pattern is found for the current node, keep iterating over
            // any later highlighting patterns that also match this node and set the match to it.
            // Captures for a given node are ordered by pattern index, so these subsequent
            // captures are guaranteed to be for highlighting, not injections or
            // local variables.
            while let Some((next_match, next_capture_index)) = layer.captures.peek() {
                let next_capture = next_match.captures[*next_capture_index];
                if next_capture.node == capture.node {
                    let following_match = layer.captures.next().unwrap().0;
                    // If the current node was found to be a local variable, then ignore
                    // the following match if it's a highlighting pattern that is disabled
                    // for local variables.
                    if (definition_highlight.is_some() || reference_highlight.is_some())
                        && layer.config.non_local_variable_patterns[following_match.pattern_index]
                    {
                        continue;
                    }
                    match_.remove();
                    capture = next_capture;
                    match_ = following_match;
                } else {
                    break;
                }
            }

            let current_highlight = layer.config.highlight_indices[capture.index as usize];

            // If this node represents a local definition, then store the current
            // highlight value on the local scope entry representing this node.
            if let Some(definition_highlight) = definition_highlight {
                *definition_highlight = current_highlight;
            }

            // Emit a scope start event and push the node's end position to the stack.
            if let Some(highlight) = reference_highlight.or(current_highlight) {
                self.last_highlight_range = Some((range.start, range.end, layer.depth));
                layer.highlight_end_stack.push(range.end);
                return self
                    .emit_event(range.start, Some(HighlightEvent::HighlightStart(highlight)));
            }

            self.sort_layers();
        }
    }
}

/* ------- Rope + TS Text Provider  ------- */
#[derive(Clone, Copy)]
struct RopeProvider<'a>(pub RopeSlice<'a>);

impl<'a> TextProvider<&'a [u8]> for RopeProvider<'a> {
    type I = ChunksBytes<'a>;

    fn text(&mut self, node: Node) -> Self::I {
        let fragment = self.0.byte_slice(node.start_byte()..node.end_byte());
        ChunksBytes {
            chunks: fragment.chunks(),
        }
    }
}

/// shim to convert rope chunks to bytes
struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}

impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}
