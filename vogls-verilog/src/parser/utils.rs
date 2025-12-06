use crate::ast::{AstId, AstIdRange, AstItem};
use crate::tokenizer::Token;

use super::token_walker::TokenRange;
use super::{AstArenas, Consumable, Diagnostics, ParserScratches, TokenWalker};

pub fn parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstId<T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, diagnostics)?;
    let end = tkw.offset;
    Ok(arenas.add(item, TokenRange { start, end }))
}

pub fn try_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
) -> Option<AstId<T>> {
    let start = tkw.offset;
    let item = T::try_consume(tkw, sc, arenas)?;
    let end = tkw.offset - 1;
    Some(arenas.add(item, TokenRange { start, end }))
}

pub fn item_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstItem<T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, diagnostics)?;
    let end = tkw.offset;
    let loc = arenas.spans.len();
    arenas.spans.push(TokenRange { start, end });
    Ok(AstItem { item, loc })
}

pub fn try_item_parse<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
) -> Option<AstItem<T>> {
    let start = tkw.offset;
    let item = T::try_consume(tkw, sc, arenas)?;
    let end = tkw.offset;
    let loc = arenas.spans.len();
    arenas.spans.push(TokenRange { start, end });
    Some(AstItem { item, loc })
}

pub fn parse_until_reaching<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    end: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let token = tkw.try_next(diagnostics.as_deref_mut())?;
        if *token.kind == end {
            break;
        }
        tkw.offset -= 1;

        let start = tkw.offset;
        let item = T::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
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
        let item = T::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_zero_or_more_delimited<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    let start = tkw.offset;
    let Some(item) = T::try_consume(tkw, sc, arenas) else {
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
        let item = T::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_zero_or_more<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    let mut has_error = false;
    while !tkw.is_empty() {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, diagnostics.as_deref_mut()) {
            Ok(item) => {
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                items.push(item);
                spans.push(token_range);
            }
            Err(err) => {
                if tkw.offset == start {
                    return Err(err);
                }
                has_error = true;
            }
        }
    }

    if has_error {
        return Err(());
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    let mut has_error = false;
    loop {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, diagnostics.as_deref_mut()) {
            Err(_) if start == tkw.offset => return Err(()),
            Err(_) => has_error = true,
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

    if has_error {
        return Err(());
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited_until_fail<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    delimiter: Token,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    let start = tkw.offset;
    let item = T::consume(tkw, sc, arenas, diagnostics)?;
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
        let save = tkw.offset;
        if !tkw.next_if_equals(delimiter) {
            break;
        }

        let start = tkw.offset;
        let Some(item) = T::try_consume(tkw, sc, arenas) else {
            tkw.offset = save;
            break;
        };
        let token_range = TokenRange {
            start,
            end: tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}
