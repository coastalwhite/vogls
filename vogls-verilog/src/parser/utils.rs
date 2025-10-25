use crate::ast::{AstId, AstIdRange};
use crate::lexer::TokenKind;
use crate::span::Span;

use super::{AstArenas, Consumable, ParseError, Parser};

pub fn parse<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Result<AstId<T>, ParseError> {
    Ok(parse_with_span::<T>(p, arenas)?.0)
}

pub fn parse_with_span<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Result<(AstId<T>, Span), ParseError> {
    let (item, span) = T::consume(p, arenas)?;
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
    end: TokenKind,
) -> Result<AstIdRange<T>, ParseError> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let token = p.tkw.try_next()?;
        if *token.kind == end {
            break;
        }
        p.tkw.offset -= 1;

        let (item, span) = T::consume(p, arenas)?;
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: TokenKind,
) -> Result<AstIdRange<T>, ParseError> {
    let (item, span) = T::consume(p, arenas)?;

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

        let (item, span) = T::consume(p, arenas)?;
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_zero_or_more_delimited<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
    delimiter: TokenKind,
) -> Result<AstIdRange<T>, ParseError> {
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

        let (item, span) = T::consume(p, arenas)?;
        items.push(item);
        spans.push(span);
    }

    Ok(arenas.add_range(items, spans))
}

pub fn parse_one_or_more<'a, T: Consumable<'a>>(
    p: &mut Parser<'a>,
    arenas: &mut AstArenas,
) -> Result<AstIdRange<T>, ParseError> {
    // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
    // here.
    let mut items = Vec::new();
    let mut spans = Vec::new();

    loop {
        let (item, span) = T::consume(p, arenas)?;
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
     delimiter: TokenKind,
 ) -> Result<AstIdRange<T>, ParseError> {
     let (item, span) = T::consume(p, arenas)?;
                                                                                              
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
