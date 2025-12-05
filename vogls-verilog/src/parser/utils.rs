use crate::ast::{AstId, AstIdRange, AstItem};
use crate::parser::ParseErrorReason;
use crate::tokenizer::Token;

use super::token_walker::TokenRange;
use super::{AstArenas, Consumable, Diagnostics, ParseErrorKind, Parser};

pub fn report_err<'a>(
    p: &mut Parser<'a>,
    diagnostics: Option<&mut Diagnostics>,
    err: ParseErrorKind,
) {
    if let Some(diagnostics) = diagnostics {
        use ParseErrorKind as K;
        let err = match err {
            K::MissingToken => ParseErrorReason::MissingToken,
            K::UnexpectedToken => {
                let t = p.tkw.get(p.tkw.offset).unwrap();
                ParseErrorReason::UnexpectedToken(*t.kind)
            }
            K::Incomplete => ParseErrorReason::Incomplete("incomplete"),
        };
        diagnostics.errors.push((TokenRange::at(p.tkw.offset), err));
    }
}

pub fn parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstId<T>, ParseErrorKind> {
    let start = p.tkw.offset;
    let item = T::consume(p, arenas, diagnostics)?;
    let end = p.tkw.offset;
    Ok(arenas.add(item, TokenRange { start, end }))
}

pub fn try_parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Option<AstId<T>> {
    let start = p.tkw.offset;
    let item = T::try_consume(p, arenas)?;
    let end = p.tkw.offset - 1;
    Some(arenas.add(item, TokenRange { start, end }))
}

pub fn item_parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstItem<T>, ParseErrorKind> {
    let start = p.tkw.offset;
    let item = T::consume(p, arenas, diagnostics)?;
    let end = p.tkw.offset;
    let loc = arenas.spans.len();
    arenas.spans.push(TokenRange { start, end });
    Ok(AstItem { item, loc })
}

pub fn try_item_parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Option<AstItem<T>> {
    let start = p.tkw.offset;
    let item = T::try_consume(p, arenas)?;
    let end = p.tkw.offset;
    let loc = arenas.spans.len();
    arenas.spans.push(TokenRange { start, end });
    Some(AstItem { item, loc })
}

pub fn parse_until_reaching<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    end: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let token = match p.tkw.try_next(diagnostics.as_deref_mut()) {
            Ok(t) => t,
            Err(err) => {
                report_err(p, diagnostics, err);
                return Err(err);
            }
        };
        if *token.kind == end {
            break;
        }
        p.tkw.offset -= 1;

        let start = p.tkw.offset;
        let item = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: p.tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    let start = p.tkw.offset;
    let item = T::consume(p, arenas, diagnostics.as_deref_mut())?;
    let token_range = TokenRange {
        start,
        end: p.tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let start = p.tkw.offset;
        let item = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: p.tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_zero_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    let start = p.tkw.offset;
    let Some(item) = T::try_consume(p, arenas) else {
        return Ok(AstIdRange::default());
    };
    let token_range = TokenRange {
        start,
        end: p.tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let start = p.tkw.offset;
        let item = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: p.tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let start = p.tkw.offset;
        let item = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        let token_range = TokenRange {
            start,
            end: p.tkw.offset,
        };
        items.push(item);
        spans.push(token_range);

        if p.tkw.is_empty() {
            break;
        }
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited_until_fail<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: Token,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    let start = p.tkw.offset;
    let item = T::consume(p, arenas, diagnostics)?;
    let token_range = TokenRange {
        start,
        end: p.tkw.offset,
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(token_range);

    loop {
        let save = p.tkw.offset;
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let start = p.tkw.offset;
        let Some(item) = T::try_consume(p, arenas) else {
            p.tkw.offset = save;
            break;
        };
        let token_range = TokenRange {
            start,
            end: p.tkw.offset,
        };
        items.push(item);
        spans.push(token_range);
    }

    Ok(arenas.add_range(items, spans))
}
