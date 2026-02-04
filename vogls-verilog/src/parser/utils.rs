use vogls_ir::token_range::TokenRange;

use crate::ast::{AstId, AstIdRange, AstItem};
use crate::tokenizer::Token;

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
    let end = tkw.offset;
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

pub fn parse_zero_or_more_while<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(&mut TokenWalker<'a>) -> bool,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    while condition(tkw) {
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

pub fn parse_one_or_more_while<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
    condition: impl Fn(&mut TokenWalker<'a>) -> bool,
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

    while condition(tkw) {
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
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ()> {
    parse_one_or_more_while(tkw, sc, arenas, diagnostics, |tkw| {
        tkw.next_if_equals(delimiter)
    })
}

pub fn parse_one_or_more_delimited_and_after<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    delimiter: Token,
    after: Token,
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
        if !tkw.is_next_equal_to(delimiter) || !tkw.is_next_nth_equal_to(1, after) {
            break;
        }
        tkw.offset += 1;

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

pub fn parse_one_or_more_delimited_one_of<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    delimiter: &[Token],
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
        if tkw
            .get(tkw.offset)
            .is_none_or(|t| !delimiter.contains(t.kind))
        {
            break;
        }
        tkw.offset += 1;

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

pub fn parse_zero_or_more_while_next<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
    condition: impl Fn(Token) -> bool,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    while tkw.get(tkw.offset).is_some_and(|t| condition(*t.kind)) {
        let start = tkw.offset;
        let Ok(item) = T::consume(tkw, sc, arenas, diagnostics.as_deref_mut()) else {
            return Err(());
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

pub fn parse_one_or_more_while_next<'a, T: Consumable<'a>>(
    tkw: &mut TokenWalker<'a>,
    sc: &mut ParserScratches,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
    mut condition: impl FnMut(Token) -> bool,
) -> Result<AstIdRange<T>, ()> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
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
                return Err(err);
            }
        }
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

    loop {
        let start = tkw.offset;
        match T::consume(tkw, sc, arenas, diagnostics.as_deref_mut()) {
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

    Ok(arenas.add_range(items, spans))
}
