use std::sync::{Arc, OnceLock, RwLock};
use temper_core::ToArcString;

pub(crate) fn init() -> temper_core::Result<()> {
    static INIT_ONCE: OnceLock<temper_core::Result<()>> = OnceLock::new();
    INIT_ONCE
        .get_or_init(|| {
            TEXT_ESCAPER__INSTANCE
                .set(TextEscaper::new())
                .unwrap_or_else(|_| panic!());
            QUOTE_ESCAPER__INSTANCE
                .set(QuoteEscaper::new())
                .unwrap_or_else(|_| panic!());
            println!("{}", greet("Ann", "Bob"));
            println!("{}", branch(true, "a"));
            println!("{}", branch(false, "b"));
            println!(
                "{}",
                each(Arc::new(vec![
                    Arc::new("c".to_string()),
                    Arc::new("d".to_string()),
                ]))
            );
            println!("{}", counted(5, "e"));
            println!("{} {}", attribute(false, "f"), attribute(true, "f"));
            println!(
                "{}",
                opened(Arc::new(vec![
                    Arc::new("g".to_string()),
                    Arc::new("h".to_string()),
                    Arc::new("i".to_string()),
                ]))
            );
            Ok(())
        })
        .clone()
}

struct MarkStruct {
    label: Arc<String>,
}

#[derive(Clone)]
pub struct Mark(Arc<MarkStruct>);

impl Mark {
    pub fn new(label: impl ToArcString) -> Mark {
        let label = label.to_arc_string();
        Mark(Arc::new(MarkStruct { label }))
    }

    pub fn label(&self) -> Arc<String> {
        self.0.label.clone()
    }
}

temper_core::impl_any_value_trait!(Mark, []);

struct BufferStruct {
    sb: Arc<std::sync::RwLock<String>>,
}

#[derive(Clone)]
pub struct Buffer(Arc<BufferStruct>);

impl Buffer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Buffer {
        Buffer(Arc::new(BufferStruct {
            sb: Arc::new(std::sync::RwLock::new(String::new())),
        }))
    }

    pub fn append(&self, x: impl ToArcString) {
        let x = x.to_arc_string();
        temper_core::string::builder::append(&self.0.sb, x.clone());
    }

    pub fn mark(&self, m: Mark) {
        temper_core::string::builder::append(&self.0.sb, Arc::new(format!("{{{}}}", m.0.label)));
    }

    pub fn to_string(&self) -> Arc<String> {
        temper_core::string::builder::to_string(&self.0.sb)
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

temper_core::impl_any_value_trait!(Buffer, []);

pub trait EffectTrait: temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync {
    fn clone_boxed(&self) -> Effect;

    fn enact_on(&self, c: Buffer);
}

#[derive(Clone)]
pub struct Effect(Arc<dyn EffectTrait>);

impl Effect {
    pub fn new(selfish: impl EffectTrait + 'static) -> Effect {
        Effect(Arc::new(selfish))
    }
}

impl EffectTrait for Effect {
    fn clone_boxed(&self) -> Effect {
        EffectTrait::clone_boxed(&*self.0)
    }

    fn enact_on(&self, c: Buffer) {
        EffectTrait::enact_on(&*self.0, c)
    }
}

temper_core::impl_any_value_trait_for_interface!(Effect);

impl std::ops::Deref for Effect {
    type Target = dyn EffectTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct AppendParseEffectStruct {
    text: Arc<String>,
}

#[derive(Clone)]
pub struct AppendParseEffect(Arc<AppendParseEffectStruct>);

impl AppendParseEffect {
    pub fn new(text: impl ToArcString) -> AppendParseEffect {
        let text = text.to_arc_string();
        AppendParseEffect(Arc::new(AppendParseEffectStruct { text }))
    }

    pub fn enact_on(&self, c: Buffer) {
        c.append(self.0.text.clone());
    }

    pub fn text(&self) -> Arc<String> {
        self.0.text.clone()
    }
}

impl EffectTrait for AppendParseEffect {
    fn clone_boxed(&self) -> Effect {
        Effect::new(self.clone())
    }

    fn enact_on(&self, c: Buffer) {
        self.enact_on(c)
    }
}

temper_core::impl_any_value_trait!(AppendParseEffect, [Effect]);

struct EventParseEffectStruct {
    event: Mark,
}

#[derive(Clone)]
pub struct EventParseEffect(Arc<EventParseEffectStruct>);

impl EventParseEffect {
    pub fn new(event: Mark) -> EventParseEffect {
        EventParseEffect(Arc::new(EventParseEffectStruct { event }))
    }

    pub fn enact_on(&self, c: Buffer) {
        c.mark(self.0.event.clone());
    }

    pub fn event(&self) -> Mark {
        self.0.event.clone()
    }
}

impl EffectTrait for EventParseEffect {
    fn clone_boxed(&self) -> Effect {
        Effect::new(self.clone())
    }

    fn enact_on(&self, c: Buffer) {
        self.enact_on(c)
    }
}

temper_core::impl_any_value_trait!(EventParseEffect, [Effect]);

struct AfterStruct {
    effects: temper_core::List<Effect>,
    state_after: bool,
}

#[derive(Clone)]
pub struct After(Arc<AfterStruct>);

impl After {
    pub fn new(effects: impl temper_core::ToList<Effect>, state_after: bool) -> After {
        let effects = effects.to_list();
        After(Arc::new(AfterStruct {
            effects,
            state_after,
        }))
    }

    pub fn effects(&self) -> temper_core::List<Effect> {
        self.0.effects.clone()
    }

    pub fn state_after(&self) -> bool {
        self.0.state_after
    }
}

temper_core::impl_any_value_trait!(After, []);

struct PropagatorStruct {
    quote: Arc<String>,
}

#[derive(Clone)]
pub struct Propagator(Arc<PropagatorStruct>);

impl Propagator {
    pub fn new(quote: impl ToArcString) -> Propagator {
        let quote = quote.to_arc_string();
        Propagator(Arc::new(PropagatorStruct { quote }))
    }

    pub fn quote(&self) -> Arc<String> {
        self.0.quote.clone()
    }
}

temper_core::impl_any_value_trait!(Propagator, []);

pub(crate) fn ignore(problem: impl ToArcString, is_error: bool) {
    let problem = problem.to_arc_string();
    if is_error {
        println!("{problem}");
    }
}

pub fn propagate_over(
    p: Propagator,
    before: bool,
    known: Option<impl ToArcString>,
    same: Arc<dyn Fn(bool, bool) -> bool + Send + Sync>,
    callout: Arc<dyn Fn(Arc<String>, bool) + Send + Sync>,
) -> After {
    let known = known.map(|x| x.to_arc_string());
    if known.is_none() {
        return After::new(Arc::new(vec![]), before);
    }
    if known.as_ref().unwrap().as_str() == "!" {
        return After::new(
            Arc::new(vec![Effect::new(EventParseEffect::new(Mark::new("bang")))]),
            before,
        );
    }
    let angles = temper_core::ListedTrait::len(&temper_core::string::split(
        known.as_ref().unwrap(),
        Arc::new("<".to_string()),
    ));
    if angles > 1 {
        callout(Arc::new("angle bracket in fixed text".to_string()), false);
    }
    if !same(before, before) {
        callout(Arc::new("states differ from themselves".to_string()), true);
    }
    let quotes = temper_core::ListedTrait::len(&temper_core::string::split(
        known.as_ref().unwrap(),
        p.0.quote.clone(),
    ))
    .wrapping_sub(1);
    After::new(
        Arc::new(vec![Effect::new(AppendParseEffect::new(
            known.clone().unwrap(),
        ))]),
        if quotes.wrapping_rem(2) == 1 {
            !before
        } else {
            before
        },
    )
}

pub trait EscaperTrait: temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync {
    fn clone_boxed(&self) -> Escaper;

    fn apply_string(&self, x: Arc<String>) -> Arc<String>;
}

#[derive(Clone)]
pub struct Escaper(Arc<dyn EscaperTrait>);

impl Escaper {
    pub fn new(selfish: impl EscaperTrait + 'static) -> Escaper {
        Escaper(Arc::new(selfish))
    }
}

impl EscaperTrait for Escaper {
    fn clone_boxed(&self) -> Escaper {
        EscaperTrait::clone_boxed(&*self.0)
    }

    fn apply_string(&self, x: Arc<String>) -> Arc<String> {
        EscaperTrait::apply_string(&*self.0, x)
    }
}

temper_core::impl_any_value_trait_for_interface!(Escaper);

impl std::ops::Deref for Escaper {
    type Target = dyn EscaperTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct TextEscaperStruct {}

#[derive(Clone)]
pub struct TextEscaper(Arc<TextEscaperStruct>);

static TEXT_ESCAPER__INSTANCE: OnceLock<TextEscaper> = OnceLock::new();

impl TextEscaper {
    #[allow(clippy::new_without_default)]
    pub fn new() -> TextEscaper {
        TextEscaper(Arc::new(TextEscaperStruct {}))
    }

    pub fn instance() -> TextEscaper {
        crate::support::initialized(|| TEXT_ESCAPER__INSTANCE.get()).clone()
    }

    pub fn apply_string(&self, x: impl ToArcString) -> Arc<String> {
        let x = x.to_arc_string();
        Arc::new(format!("[{x}]"))
    }
}

impl EscaperTrait for TextEscaper {
    fn clone_boxed(&self) -> Escaper {
        Escaper::new(self.clone())
    }

    fn apply_string(&self, x: Arc<String>) -> Arc<String> {
        self.apply_string(x)
    }
}

temper_core::impl_any_value_trait!(TextEscaper, [Escaper]);

struct QuoteEscaperStruct {}

#[derive(Clone)]
pub struct QuoteEscaper(Arc<QuoteEscaperStruct>);

static QUOTE_ESCAPER__INSTANCE: OnceLock<QuoteEscaper> = OnceLock::new();

impl QuoteEscaper {
    #[allow(clippy::new_without_default)]
    pub fn new() -> QuoteEscaper {
        QuoteEscaper(Arc::new(QuoteEscaperStruct {}))
    }

    pub fn instance() -> QuoteEscaper {
        crate::support::initialized(|| QUOTE_ESCAPER__INSTANCE.get()).clone()
    }

    pub fn apply_string(&self, x: impl ToArcString) -> Arc<String> {
        let x = x.to_arc_string();
        Arc::new(format!("({x})"))
    }
}

impl EscaperTrait for QuoteEscaper {
    fn clone_boxed(&self) -> Escaper {
        Escaper::new(self.clone())
    }

    fn apply_string(&self, x: Arc<String>) -> Arc<String> {
        self.apply_string(x)
    }
}

temper_core::impl_any_value_trait!(QuoteEscaper, [Escaper]);

struct BothStruct {
    first: Escaper,
    second: Escaper,
}

#[derive(Clone)]
pub struct Both(Arc<BothStruct>);

impl Both {
    pub fn new(first: Escaper, second: Escaper) -> Both {
        Both(Arc::new(BothStruct { first, second }))
    }

    pub fn apply_string(&self, x: impl ToArcString) -> Arc<String> {
        let x = x.to_arc_string();
        self.0
            .second
            .apply_string(self.0.first.apply_string(x.clone()))
    }

    pub fn first(&self) -> Escaper {
        self.0.first.clone()
    }

    pub fn second(&self) -> Escaper {
        self.0.second.clone()
    }
}

impl EscaperTrait for Both {
    fn clone_boxed(&self) -> Escaper {
        Escaper::new(self.clone())
    }

    fn apply_string(&self, x: Arc<String>) -> Arc<String> {
        self.apply_string(x)
    }
}

temper_core::impl_any_value_trait!(Both, [Escaper]);

struct PickerStruct {
    strict: bool,
}

#[derive(Clone)]
pub struct Picker(Arc<PickerStruct>);

impl Picker {
    pub fn new(strict: bool) -> Picker {
        Picker(Arc::new(PickerStruct { strict }))
    }

    pub fn escaper_for(
        &self,
        in_quote: bool,
        callout: Arc<dyn Fn(Arc<String>, bool) + Send + Sync>,
    ) -> Escaper {
        if self.0.strict && in_quote {
            callout(Arc::new("a value in quotes".to_string()), false);
        }
        if in_quote {
            Escaper::new(Both::new(
                Escaper::new(TextEscaper::instance()),
                Escaper::new(QuoteEscaper::instance()),
            ))
        } else {
            Escaper::new(TextEscaper::instance())
        }
    }

    pub fn strict(&self) -> bool {
        self.0.strict
    }
}

temper_core::impl_any_value_trait!(Picker, []);

pub trait ContextualAutoescapingAccumulatorTrait:
    temper_core::AsAnyValue + temper_core::AnyValueTrait + Send + Sync
{
    fn clone_boxed(&self) -> ContextualAutoescapingAccumulator;

    fn accumulated(&self) -> Arc<String>;
}

#[derive(Clone)]
pub struct ContextualAutoescapingAccumulator(Arc<dyn ContextualAutoescapingAccumulatorTrait>);

impl ContextualAutoescapingAccumulator {
    pub fn new(
        selfish: impl ContextualAutoescapingAccumulatorTrait + 'static,
    ) -> ContextualAutoescapingAccumulator {
        ContextualAutoescapingAccumulator(Arc::new(selfish))
    }
}

impl ContextualAutoescapingAccumulatorTrait for ContextualAutoescapingAccumulator {
    fn clone_boxed(&self) -> ContextualAutoescapingAccumulator {
        ContextualAutoescapingAccumulatorTrait::clone_boxed(&*self.0)
    }

    fn accumulated(&self) -> Arc<String> {
        ContextualAutoescapingAccumulatorTrait::accumulated(&*self.0)
    }
}

temper_core::impl_any_value_trait_for_interface!(ContextualAutoescapingAccumulator);

impl std::ops::Deref for ContextualAutoescapingAccumulator {
    type Target = dyn ContextualAutoescapingAccumulatorTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct QuotingStruct {
    in_quote: RwLock<bool>,
    collector: Buffer,
}

#[derive(Clone)]
pub struct Quoting(Arc<QuotingStruct>);

impl Quoting {
    pub fn new_collector() -> Buffer {
        Buffer::new()
    }

    pub fn initial_state() -> bool {
        false
    }

    pub fn propagator() -> Propagator {
        Propagator::new("'")
    }

    pub fn picker() -> Picker {
        Picker::new(false)
    }

    pub fn same_state_predicate() -> Arc<dyn Fn(bool, bool) -> bool + Send + Sync> {
        Arc::new(move |a: bool, b: bool| a == b)
    }

    pub fn merge_states(
        a: bool,
        b: bool,
        callout: Arc<dyn Fn(Arc<String>, bool) + Send + Sync>,
    ) -> bool {
        if a != b {
            callout(
                Arc::new("branches end in different contexts".to_string()),
                true,
            );
        }
        a
    }

    pub fn check_end_state(in_quote: bool, callout: Arc<dyn Fn(Arc<String>, bool) + Send + Sync>) {
        if in_quote {
            callout(Arc::new("ends inside quotes".to_string()), true);
        }
    }

    pub fn from_collector(c: Buffer) -> Arc<String> {
        c.to_string()
    }

    pub fn enact(m: Mark, c: Buffer) {
        c.mark(m.clone());
    }

    #[allow(clippy::new_without_default)]
    pub fn new() -> Quoting {
        Quoting(Arc::new(QuotingStruct {
            in_quote: RwLock::new(false),
            collector: Buffer::new(),
        }))
    }

    pub fn append_safe(&self, x: impl ToArcString) {
        let x = x.to_arc_string();
        let after = propagate_over(
            Quoting::propagator(),
            self.in_quote(),
            Some(x.clone()),
            Quoting::same_state_predicate(),
            Arc::new(|a0: Arc<String>, a1: bool| ignore(a0, a1)),
        );
        *self.0.in_quote.write().unwrap() = after.0.state_after;
        let list: temper_core::List<Effect> = after.0.effects.clone();
        let length = temper_core::ListedTrait::len(&list);
        let mut i: i32 = 0;
        while i < length {
            let e = temper_core::ListedTrait::get(&list, i);
            i = i.wrapping_add(1);
            e.enact_on(self.0.collector.clone());
        }
    }

    pub fn append(&self, x: impl ToArcString) {
        let x = x.to_arc_string();
        let escaped = Quoting::picker()
            .escaper_for(
                self.in_quote(),
                Arc::new(|a0: Arc<String>, a1: bool| ignore(a0, a1)),
            )
            .apply_string(x.clone());
        self.0.collector.append(escaped.clone());
    }

    pub fn accumulated(&self) -> Arc<String> {
        self.0.collector.to_string()
    }

    fn in_quote(&self) -> bool {
        *self.0.in_quote.read().unwrap()
    }
}

impl ContextualAutoescapingAccumulatorTrait for Quoting {
    fn clone_boxed(&self) -> ContextualAutoescapingAccumulator {
        ContextualAutoescapingAccumulator::new(self.clone())
    }

    fn accumulated(&self) -> Arc<String> {
        self.accumulated()
    }
}

temper_core::impl_any_value_trait!(Quoting, [ContextualAutoescapingAccumulator]);

pub(crate) fn greet(name: impl ToArcString, who: impl ToArcString) -> Arc<String> {
    let name = name.to_arc_string();
    let who = who.to_arc_string();
    let collector = Quoting::new_collector();
    {
        collector.append("say 'hi ");
        collector.append(
            QuoteEscaper::instance()
                .apply_string(TextEscaper::instance().apply_string(name.clone())),
        );
        collector.append("' to ");
        collector.append(TextEscaper::instance().apply_string(who.clone()));
        Quoting::enact(Mark::new("bang"), collector.clone());
    }
    Quoting::from_collector(collector.clone())
}

pub(crate) fn branch(c: bool, x: impl ToArcString) -> Arc<String> {
    let x = x.to_arc_string();
    let collector = Quoting::new_collector();
    {
        collector.append(TextEscaper::instance().apply_string(x.clone()));
        if c {
            collector.append("''");
        }
        collector.append(" '");
        collector.append(
            QuoteEscaper::instance().apply_string(TextEscaper::instance().apply_string(x.clone())),
        );
        collector.append("'");
    }
    Quoting::from_collector(collector.clone())
}

pub(crate) fn each(xs: impl temper_core::ToList<Arc<String>>) -> Arc<String> {
    let xs = xs.to_list();
    let collector = Quoting::new_collector();
    {
        {
            let length = temper_core::ListedTrait::len(&xs);
            let mut i: i32 = 0;
            while i < length {
                let x = temper_core::ListedTrait::get(&xs, i);
                i = i.wrapping_add(1);
                collector.append("'");
                collector.append(
                    QuoteEscaper::instance()
                        .apply_string(TextEscaper::instance().apply_string(x.clone())),
                );
                collector.append("' ");
                collector.append(TextEscaper::instance().apply_string(x.clone()));
                collector.append(";");
            }
        }
    }
    Quoting::from_collector(collector.clone())
}

pub(crate) fn counted(n: i32, x: impl ToArcString) -> Arc<String> {
    let x = x.to_arc_string();
    let collector = Quoting::new_collector();
    {
        let mut i: i32 = 0;
        while i < n {
            i = i.wrapping_add(1);
            if i == 2 {
                continue;
            }
            collector.append("'");
            collector.append(
                QuoteEscaper::instance()
                    .apply_string(TextEscaper::instance().apply_string(x.clone())),
            );
            collector.append("'");
            if i == 4 {
                break;
            }
            collector.append(TextEscaper::instance().apply_string(x.clone()));
            collector.append(",");
        }
    }
    Quoting::from_collector(collector.clone())
}

pub(crate) fn attribute(c: bool, x: impl ToArcString) -> Arc<String> {
    let x = x.to_arc_string();
    let collector = Quoting::new_collector();
    {
        collector.append("a='");
        if c {
            return Arc::new("none".to_string());
        }
        collector.append(
            QuoteEscaper::instance().apply_string(TextEscaper::instance().apply_string(x.clone())),
        );
        collector.append("'");
    }
    Quoting::from_collector(collector.clone())
}

pub(crate) fn opened(xs: impl temper_core::ToList<Arc<String>>) -> Arc<String> {
    let xs = xs.to_list();
    let accumulator = Quoting::new();
    {
        {
            let length = temper_core::ListedTrait::len(&xs);
            let mut i: i32 = 0;
            while i < length {
                let x = temper_core::ListedTrait::get(&xs, i);
                i = i.wrapping_add(1);
                accumulator.append_safe("'");
                accumulator.append(x.clone());
            }
        }
    }
    accumulator.accumulated()
}

pub mod builders {
    use super::*;

    #[derive(Clone)]
    pub struct AfterBuilder {
        pub effects: temper_core::List<Effect>,
        pub state_after: bool,
    }

    impl AfterBuilder {
        pub fn build(self) -> After {
            After::new(self.effects, self.state_after)
        }
    }

    #[derive(Clone)]
    pub struct BothBuilder {
        pub first: Escaper,
        pub second: Escaper,
    }

    impl BothBuilder {
        pub fn build(self) -> Both {
            Both::new(self.first, self.second)
        }
    }
}
