use crate::ast::{AstId, AstIdRange};
use crate::parser::ParseErrorReason;
use crate::span::Span;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParseError, ParseErrorKind, Parser};

pub fn report_err<'a>(
    p: &mut Parser<'a>,
    diagnostics: Option<&mut Diagnostics>,
    err: ParseErrorKind,
) {
    if let Some(diagnostics) = diagnostics {
        use ParseErrorKind as K;
        let (span, err) = match err {
            K::MissingToken => (p.tkw.span_at_cursor(), ParseErrorReason::MissingToken),
            K::UnexpectedToken => {
                let t = p.tkw.get(p.tkw.offset).unwrap();
                (*t.span, ParseErrorReason::UnexpectedToken(*t.kind))
            }
            K::Incomplete => (
                p.tkw.span_at_cursor(),
                ParseErrorReason::Incomplete("incomplete"),
            ),
        };
        diagnostics.errors.push((span, err));
    }
}

pub fn parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<AstId<T>, ParseErrorKind> {
    Ok(parse_with_span::<T>(p, arenas, diagnostics)?.0)
}

pub fn parse_with_span<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<(AstId<T>, Span), ParseErrorKind> {
    let (item, span) = T::consume(p, arenas, diagnostics)?;
    Ok((arenas.add(item, span), span))
}

pub fn try_parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Option<AstId<T>> {
    Some(try_parse_with_span::<T>(p, arenas)?.0)
}

pub fn try_parse_with_span<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Option<(AstId<T>, Span)> {
    let (item, span) = T::try_consume(p, arenas)?;
    Some((arenas.add(item, span), span))
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

        let (item, span) = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    let (item, span) = T::consume(p, arenas, diagnostics.as_deref_mut())?;

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(span);

    loop {
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let (item, span) = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_zero_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: Token,
    mut diagnostics: Option<&mut Diagnostics>,
) -> Result<AstIdRange<T>, ParseErrorKind> {
    let Some((item, span)) = T::try_consume(p, arenas) else {
        return Ok(AstIdRange::default());
    };

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(span);

    loop {
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let (item, span) = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        items.push(item);
        spans.push(span);
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
        let (item, span) = T::consume(p, arenas, diagnostics.as_deref_mut())?;
        items.push(item);
        spans.push(span);

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
    let (item, span) = T::consume(p, arenas, diagnostics)?;

    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();
    items.push(item);
    spans.push(span);

    loop {
        let save = p.tkw.offset;
        if !p.tkw.next_if_equals(delimiter) {
            break;
        }

        let Some((item, span)) = T::try_consume(p, arenas) else {
            p.tkw.offset = save;
            break;
        };
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}
