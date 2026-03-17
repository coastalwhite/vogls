use vogls_ir::token_range::TokenRange;

use crate::arena::Arena;
use crate::ast::{AstId, AstIdRange, AstItem};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParserScratches, TokenWalker};

pub fn push<'a, T>(
    arenas: &mut AstArenas,
    ast: &'a Arena,
    item: T,
    tr: TokenRange,
) -> AstId<'a, T> {
    let ptr = ast.add(item);
    let loc = arenas.add_tr(tr);
    AstId { node: ptr, loc }
}

pub fn push_range<'a, T, II: IntoIterator<Item = T>, IT: IntoIterator<Item = TokenRange>>(
    arenas: &mut AstArenas,
    ast: &'a Arena,
    items: II,
    trs: IT,
) -> AstIdRange<'a, T>
where
    II::IntoIter: ExactSizeIterator,
    IT::IntoIter: ExactSizeIterator,
{
    let items = items.into_iter();
    let trs = trs.into_iter();
    debug_assert_eq!(items.len(), trs.len());
    let ptr = ast.extend(items);
    let loc = arenas.add_tr_range(trs);
    AstIdRange { node: ptr, loc }
}

pub fn parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstId<'a, T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, ast, diagnostics)?;
    let end = tkw.offset;
    Ok(push(arenas, ast, item, TokenRange { start, end }))
}

pub fn try_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
) -> Option<AstId<'a, T>> {
    let start = tkw.offset;
    let item = T::try_consume(tkw, sc, arenas, ast)?;
    let end = tkw.offset;
    Some(push(arenas, ast, item, TokenRange { start, end }))
}

pub fn item_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstItem<T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, ast, diagnostics)?;
    let end = tkw.offset;
    let loc = arenas.add_tr(TokenRange { start, end });
    Ok(AstItem { item, loc })
}

pub fn try_item_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
) -> Option<AstItem<T>> {
    let start = tkw.offset;
    let item = T::try_consume(tkw, sc, arenas, ast)?;
    let end = tkw.offset;
    let loc = arenas.add_tr(TokenRange { start, end });
    Some(AstItem { item, loc })
}

pub fn parse_zero_or_more_while<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(&mut TokenWalker<'_>) -> bool,
) -> Result<AstIdRange<'a, T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    while condition(tkw) {
        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_one_or_more_while<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(&mut TokenWalker<'_>) -> bool,
) -> Result<AstIdRange<'a, T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
    let token_range = TokenRange {
        start,
        end: tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    while condition(tkw) {
        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_one_or_more_delimited<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    delimiter: Token,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    parse_one_or_more_while(tkw, sc, arenas, ast, diagnostics, |tkw| {
        tkw.next_if_equals(delimiter)
    })
}

pub fn parse_one_or_more_delimited_and_after<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    delimiter: Token,
    after: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
    let token_range = TokenRange {
        start,
        end: tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        if !tkw.is_next_equal_to(delimiter) || !tkw.is_next_nth_equal_to(1, after) {
            break;
        }
        tkw.offset += 1;

        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_one_or_more_delimited_one_of<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    delimiter: &[Token],
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
    let token_range = TokenRange {
        start,
        end: tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        if tkw
            .get(tkw.offset)
            .is_none_or(|t| !delimiter.contains(t.kind))
        {
            break;
        }
        tkw.offset += 1;

        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_zero_or_more_delimited<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    let start = tkw.offset;
    let Some(item) = T::try_consume(tkw, sc, arenas, ast) else {
        return Ok(AstIdRange::default());
    };
    let token_range = TokenRange {
        start,
        end: tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        if !tkw.next_if_equals(delimiter) {
            break;
        }

        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_zero_or_more_while_next<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(Token) -> bool,
) -> Result<AstIdRange<'a, T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    while tkw.get(tkw.offset).is_some_and(|t| condition(*t.kind)) {
        let start = tkw.offset;
        let Ok(item) = T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut()) else {
            return Err(());
        };
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_one_or_more_while_next<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(Token) -> bool,
) -> Result<AstIdRange<'a, T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut()) {
            Ok(item) => {
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                items.push(item);
                spans.push(token_range);
            }
            Err(err) => {
                return Err(err);
            }
        }

        let Some(t) = tkw.get(tkw.offset) else {
            break;
        };
        let t = *t.kind;
        if !condition(t) {
            break;
        }
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_zero_or_more<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    while !tkw.is_empty() {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut()) {
            Ok(item) => {
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                items.push(item);
                spans.push(token_range);
            }
            Err(err) => {
                return Err(err);
            }
        }
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}

pub fn parse_one_or_more<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'_>,
    sc: &mut ParserScratches<'a>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<'a, T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut()) {
            Err(_) => return Err(()),
            Ok(item) => {
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                items.push(item);
                spans.push(token_range);
            }
        }

        if tkw.is_empty() {
            break;
        }
    }

    let items = ast.extend(items);
    let loc = arenas.add_tr_range(spans);
    Ok(AstIdRange { node: items, loc })
}
