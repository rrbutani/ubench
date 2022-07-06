use core::fmt::{self, Write};

use owo_colors::{OwoColorize, Style};

use super::io::{Output, OutputAdapter, Void};
use super::Reporter;
use crate::metrics::{Metric, MetricFmtAdapter};

pub struct BasicReporter<'o, Out: Output + ?Sized, U = ()> {
    out: OutputAdapter<'o, Out>,
    iterations: usize,
    pub format_options: FormatOptions,
    state: State<U>,
}

#[allow(clippy::needless_lifetimes)]
impl BasicReporter<'static, Void> {
    // `core::fmt::Write` impl as trait object
    // `embedded_hal::serial::Write<W: From<u8>>` impl as `&mut dyn core::fmt::Write` as trait object
    // `std::io::Write` impl as trait object
    //
    // `embedded_hal::serial::Write<u8>` impl directly
    pub fn new<'o, U>(out: &'o mut impl Output) -> BasicReporter<'o, impl Output, U> {
        BasicReporter {
            out: OutputAdapter(out),
            iterations: 0,
            format_options: Default::default(),
            state: Default::default(),
        }
    }

    #[cfg(feature = "embedded-hal")]
    #[cfg_attr(all(docs, not(doctest)), doc(cfg(feature = "embedded-hal")))]
    pub fn new_with_serial<'o, W, S, U>(out: &'o mut S) -> BasicReporter<'o, S, U>
    where
        S: embedded_hal::serial::Write<u8>,
    {
        BasicReporter {
            out: OutputAdapter(out),
            iterations: 0,
            format_options: Default::default(),
            state: Default::default(),
        }
    }

    pub fn new_with_fmt_write<'o, Fw: fmt::Write, U>(
        out: &'o mut Fw,
    ) -> BasicReporter<'o, dyn fmt::Write + 'o, U> {
        BasicReporter {
            out: OutputAdapter(out),
            iterations: 0,
            format_options: Default::default(),
            state: Default::default(),
        }
    }

    #[cfg(feature = "std")]
    #[cfg_attr(all(docs, not(doctest)), doc(cfg(feature = "std")))]
    pub fn new_with_io_write<'o, Iw: std::io::Write, U>(
        out: &'o mut Iw,
    ) -> BasicReporter<'o, dyn std::io::Write + 'o, U> {
        BasicReporter {
            out: OutputAdapter(out),
            iterations: 0,
            format_options: Default::default(),
            state: Default::default(),
        }
    }
}

impl<'o, O: Output + ?Sized, U> BasicReporter<'o, O, U> {
    pub fn set_format_options(mut self, options: FormatOptions) -> Self {
        self.format_options = options;
        self
    }
}

pub struct FormatOptions {
    pub prefix: Option<fn(&mut dyn Write) -> fmt::Result>,
    pub single_box_style: Style,
    pub single_box_spec: support::BoxSpec,
    pub suite_box_style: Style,
    pub suite_box_spec: support::BoxSpec,
    pub iteration_count_style: Style,
    pub top_level_bench_name_style: Style,
    pub input_style: Style,
    pub unit_style: Style,
    pub avg_style: Style,
    pub range_style: Style,
    pub min_style: Style,
    pub max_style: Style,
    pub sub_bench_name_style: Style,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            prefix: Some(|f| f.write_str("┆ ")),
            single_box_style: Style::new().blue(),
            single_box_spec: support::SINGLE_LINED_BOX,
            suite_box_style: Style::new().green(),
            suite_box_spec: support::DOUBLE_LINED_BOX,
            iteration_count_style: Style::new(),
            top_level_bench_name_style: Style::new().bold(),
            input_style: Style::new().magenta(),
            unit_style: Style::new().bold(),
            avg_style: Style::new().green().bold(),
            range_style: Style::new().dimmed(),
            min_style: Style::new().yellow(),
            max_style: Style::new().red(),
            sub_bench_name_style: Style::new().cyan().italic(),
        }
    }
}

#[derive(Debug)]
enum State<U> {
    WaitingForNextTopLevel,

    WaitingForNextSingleBenchmark {
        est_num_inputs: usize,
    },
    RunningSingleBenchmark {
        remaining_iterations: usize,
        current_min: U,
        current_max: U,
        current_sum: U,
        est_num_inputs: usize,
    },

    WaitingForNextInputInSuite {
        suite_size: usize,
        benchmark_name_max_width: usize,
        est_num_inputs: usize,
    },
    SuiteWaitingForNextBenchmarkForInput {
        suite_size: usize,
        benchmark_name_max_width: usize,
        est_num_inputs: usize,
        remaining_benchmarks_for_input: usize,
    },
    RunningBenchmarkInSuite {
        suite_size: usize,
        benchmark_name_max_width: usize,
        est_num_inputs: usize,
        remaining_benchmarks_for_input: usize,

        remaining_iterations: usize,
        current_min: U,
        current_max: U,
        current_sum: U,
    },
}

impl<U> Default for State<U> {
    fn default() -> Self {
        State::WaitingForNextTopLevel
    }
}

mod support {
    use core::fmt::{self, Display, Write};
    use owo_colors::{OwoColorize, Style};

    // When `unicode-width` is not enabled, this is bad and ignores the fact
    // that printed chars (i.e. emoji) can be wide.
    pub(crate) fn estimated_str_width(s: &str) -> usize {
        #[cfg(not(feature = "unicode-width"))]
        let res = s.chars().count();

        #[cfg(feature = "unicode-width")]
        let res = {
            use unicode_width::UnicodeWidthStr;
            UnicodeWidthStr::width(s)
        };

        res
    }

    // TODO: replace with `usize::log` when that becomes stable...
    pub(crate) fn estimated_num_width(n: usize) -> usize {
        match n {
            0..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=99999 => 5,
            100000..=999999 => 6,
            1000000..=9999999 => 7,
            10000000..=99999999 => 8,
            100000000..=999999999 => 9,
            _ => 20,
        }
    }

    pub(crate) struct Repeat<T>(T, usize);
    impl<T: Display> Display for Repeat<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for _ in 0..self.1 {
                self.0.fmt(f)?
            }

            Ok(())
        }
    }

    pub(crate) struct Joined<A, B>(A, B);
    impl<A: Display, B: Display> Display for Joined<A, B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)?;
            self.1.fmt(f)
        }
    }
    pub(crate) trait FmtUtil: Sized {
        fn join<O>(self, other: O) -> Joined<Self, O> {
            Joined(self, other)
        }

        fn repeat(self, times: usize) -> Repeat<Self> {
            Repeat(self, times)
        }
    }
    impl<A> FmtUtil for A {}

    #[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
    pub struct BoxSpec {
        top_left: char,
        top_right: char,
        bot_left: char,
        bot_right: char,
        vertical: char,
        horizontal: char,
    }

    pub const SINGLE_LINED_BOX: BoxSpec = BoxSpec {
        top_left: '┌',
        top_right: '┐',
        bot_left: '└',
        bot_right: '┘',
        vertical: '│',
        horizontal: '─',
    };

    pub const DOUBLE_LINED_BOX: BoxSpec = BoxSpec {
        top_left: '╔',
        top_right: '╗',
        bot_left: '╚',
        bot_right: '╝',
        vertical: '║',
        horizontal: '═',
    };

    pub(crate) fn draw_boxed<W: Write>(
        f: &mut W,
        mut prefix: Option<impl FnMut(&mut dyn Write) -> fmt::Result>,
        spec: &BoxSpec,
        content: &str,
        box_style: Style,
        content_style: Style,
    ) {
        let lines = content.split_terminator('\n');
        let width = lines.clone().map(estimated_str_width).max().unwrap_or(0);

        macro_rules! line {
            ($(
                ($($tt:tt)+)
            ),* $(,)?) => {
                if let Some(ref mut p) = prefix {
                    p(f).unwrap()
                }

                $(
                    write!(f, $($tt)+).unwrap();
                )*

                writeln!(f).unwrap();
            };
        }

        // Top:
        line! {
            ("{}", spec.top_left.join(spec.horizontal.repeat(width + 2))
                .join(spec.top_right)
                .style(box_style)
            ),
        }

        // Content:
        for l in lines {
            line! {
                ("{} ", spec.vertical.style(box_style)),
                ("{}{}", l.style(content_style), " ".repeat(width - estimated_str_width(l))),
                (" {}", spec.vertical.style(box_style)),
            }
        }

        // End:
        line! {
            ("{}", spec.bot_left.join(spec.horizontal.repeat(width + 2))
                    .join(spec.bot_right)
                    .style(box_style)
            ),
        }
    }
}
use support::*;

macro_rules! prefixed {
    (($self:ident) <- $(
        ($($tt:tt)+)
    ),* $(,)?) => {
        if let Some(ref mut p) = $self.format_options.prefix {
            p(&mut $self.out).unwrap()
        }

        prefixed![($self) ++  $(
            ($($tt)*),
        )*];
    };

    // Omit prefix!
    (($self:ident) ++ $(
        ($($tt:tt)+)
    ),* $(,)?) => {
        $(
            write!($self.out, $($tt)+).unwrap();
        )*
    }
}

impl<'o, O: Output + ?Sized, U> BasicReporter<'o, O, U> {
    fn print_stats<M: Metric<Unit = U>>(&mut self, indent: usize, sum: U, max: U, min: U)
    where
        // rustc can't prove these are already satisfied by the `M: Metric<Unit
        // = U>` impl, for some reason...
        U: core::ops::Div<M::Divisor, Output = U>,
        U: core::ops::Sub<Output = U>,
        U: Ord,
        U: Copy,
    {
        let avg: M::Unit = {
            let count: M::Divisor = self.iterations.try_into().map_err(|_| ()).unwrap();
            sum / count
        };
        let range = {
            let upper = max - avg;
            let lower = avg - min;

            upper.max(lower)
        };
        prefixed![(self) ++
            ("{}", " ".repeat(indent)),
            ("{} ± {} ",
                MetricFmtAdapter::<M>(&avg).style(self.format_options.avg_style),
                MetricFmtAdapter::<M>(&range).style(self.format_options.range_style),
            ),
            ("{}[{} {} {}]{}",
                "(".dimmed(),
                MetricFmtAdapter::<M>(&min).style(self.format_options.min_style),
                "to".dimmed(),
                MetricFmtAdapter::<M>(&max).style(self.format_options.max_style),
                ")".dimmed(),
            ),
            ("\n")
        ];
    }
}

impl<'o, O, M> Reporter<M> for BasicReporter<'o, O, M::Unit>
where
    O: Output + ?Sized,
    M: Metric,
{
    fn top_level_benchmarks<I: Iterator<Item = &'static str> + Clone>(&mut self, _names: I) {}
    fn num_iterations(&mut self, iterations: usize) {
        debug_assert!(iterations > 0);
        self.iterations = iterations;
    }

    fn starting_single_benchmark(
        &mut self,
        name: &'static str,
        input_size_hint: (usize, Option<usize>),
    ) {
        debug_assert!(matches!(self.state, State::WaitingForNextTopLevel));
        self.state = State::WaitingForNextSingleBenchmark {
            est_num_inputs: input_size_hint.1.unwrap_or(input_size_hint.0),
        };

        draw_boxed(
            &mut self.out,
            self.format_options.prefix,
            &self.format_options.single_box_spec,
            name,
            self.format_options.single_box_style,
            self.format_options.top_level_bench_name_style,
        );
        prefixed![(self) <- ("\n")];
        prefixed![(self) <- (
            "{}{}{}\n",
            "Inputs (".dimmed(),
            self.iterations.style(self.format_options.iteration_count_style),
            " iterations each, measuring ".dimmed(),
            M::UNIT_NAME.style(self.format_options.unit_style),
            ")".dimmed(),
        )];
    }

    fn single_benchmark_run(
        &mut self,
        input_idx: usize,
        input: &dyn fmt::Debug,
        iteration_idx: usize,
        measurement: M::Unit,
    ) {
        use State::*;
        match &mut self.state {
            WaitingForNextSingleBenchmark { est_num_inputs } => {
                self.state = RunningSingleBenchmark {
                    remaining_iterations: self.iterations - 1,
                    current_min: measurement,
                    current_max: measurement,
                    current_sum: measurement,
                    est_num_inputs: *est_num_inputs,
                };
            }
            RunningSingleBenchmark {
                remaining_iterations,
                current_min,
                current_max,
                current_sum,
                ..
            } => {
                *remaining_iterations -= 1;
                *current_min = (*current_min).min(measurement);
                *current_max = (*current_max).max(measurement);
                *current_sum = *current_sum + measurement;

                debug_assert_eq!(*remaining_iterations + iteration_idx + 1, self.iterations);
            }
            _ => unreachable!(),
        }

        if let RunningSingleBenchmark {
            remaining_iterations: 0,
            current_min,
            current_max,
            current_sum,
            est_num_inputs,
        } = self.state
        {
            // We're done with this input!

            // First print the input:
            let input_num_width = estimated_num_width(est_num_inputs);
            prefixed![(self) <-
                (" "),
                ("{: >num_width$}{} ", input_idx + 1, '.'.dimmed(), num_width = input_num_width),
                ("{}{:?}{}", '`'.dimmed(), input.style(self.format_options.input_style), '`'.dimmed()),
                ("\n"),
            ];

            // Next print the stats:
            prefixed![(self) <- (" ")];
            self.print_stats::<M>(input_num_width + 2, current_sum, current_max, current_min);

            // We'll either get another input or we'll end the single benchmark.
            self.state = State::WaitingForNextSingleBenchmark { est_num_inputs };
        }
    }

    fn ending_single_benchmark(&mut self, _name: &'static str) {
        debug_assert!(matches!(
            self.state,
            State::WaitingForNextSingleBenchmark { .. }
        ));
        self.state = State::WaitingForNextTopLevel;

        prefixed![(self) <- ("\n")];
        writeln!(self.out, "\n\n").unwrap();
    }

    fn starting_new_benchmark_suite<I: Iterator<Item = &'static str> + Clone>(
        &mut self,
        name: &'static str,
        input_size_hint: (usize, Option<usize>),
        benchmark_names: I,
    ) {
        debug_assert!(matches!(self.state, State::WaitingForNextTopLevel));
        self.state = State::WaitingForNextInputInSuite {
            suite_size: benchmark_names.clone().count(),
            benchmark_name_max_width: benchmark_names.map(estimated_str_width).max().unwrap_or(0),
            est_num_inputs: input_size_hint.1.unwrap_or(input_size_hint.0),
        };

        draw_boxed(
            &mut self.out,
            self.format_options.prefix,
            &self.format_options.suite_box_spec,
            name,
            self.format_options.suite_box_style,
            self.format_options.top_level_bench_name_style,
        );
        prefixed![(self) <- ("\n")];
        prefixed![(self) <- (
            "{}{}{}\n",
            "Inputs (".dimmed(),
            self.iterations.style(self.format_options.iteration_count_style),
            " iterations each):".dimmed(),
        )];
    }

    fn suite_benchmark_run(
        &mut self,
        input_idx: usize,
        input: &dyn fmt::Debug,
        benchmark_idx: usize,
        benchmark_name: &'static str,
        iteration_idx: usize,
        measurement: M::Unit,
    ) {
        use State::*;

        // First, handle the case where we just started a new input in the suite:
        match self.state {
            WaitingForNextInputInSuite {
                suite_size,
                benchmark_name_max_width,
                est_num_inputs,
            } => {
                debug_assert_eq!(benchmark_idx, 0);
                self.state = SuiteWaitingForNextBenchmarkForInput {
                    suite_size,
                    benchmark_name_max_width,
                    est_num_inputs,
                    remaining_benchmarks_for_input: suite_size,
                };

                // Print the input:
                let input_num_width = estimated_num_width(est_num_inputs);
                prefixed![(self) <-
                    (" "),
                    ("{: >num_width$}{} ", input_idx + 1, '.'.dimmed(), num_width = input_num_width),
                    ("{}{:?}{}", '`'.dimmed(), input.style(self.format_options.input_style), '`'.dimmed()),
                    ("\n"),
                ];
            }
            SuiteWaitingForNextBenchmarkForInput { .. } | RunningBenchmarkInSuite { .. } => {
                /* handled below */
            }
            _ => unreachable!(),
        }

        // Next, handle the case where we just started a new benchmark for an input:
        match &mut self.state {
            SuiteWaitingForNextBenchmarkForInput {
                suite_size,
                benchmark_name_max_width,
                est_num_inputs,
                remaining_benchmarks_for_input,
            } => {
                debug_assert_eq!(iteration_idx, 0);
                debug_assert_eq!(*remaining_benchmarks_for_input + benchmark_idx, *suite_size);
                self.state = RunningBenchmarkInSuite {
                    suite_size: *suite_size,
                    benchmark_name_max_width: *benchmark_name_max_width,
                    est_num_inputs: *est_num_inputs,
                    remaining_benchmarks_for_input: *remaining_benchmarks_for_input - 1,
                    remaining_iterations: self.iterations - 1,
                    current_min: measurement,
                    current_max: measurement,
                    current_sum: measurement,
                }
            }
            // If we were already running a benchmark:
            RunningBenchmarkInSuite {
                remaining_iterations,
                current_min,
                current_max,
                current_sum,
                ..
            } => {
                *remaining_iterations -= 1;
                *current_min = (*current_min).min(measurement);
                *current_max = (*current_max).max(measurement);
                *current_sum = *current_sum + measurement;

                debug_assert_eq!(*remaining_iterations + iteration_idx + 1, self.iterations);
            }
            _ => unreachable!(),
        }

        // Now, handle the case where we have finished all the iterations for a
        // particular (input, benchmark) pair:
        if let RunningBenchmarkInSuite {
            remaining_iterations: 0,
            current_min,
            current_max,
            current_sum,
            suite_size,
            benchmark_name_max_width,
            est_num_inputs,
            remaining_benchmarks_for_input,
        } = self.state
        {
            // We're done with this benchmark which means its time to print a line!

            // First print the benchmark's name, right aligned:
            let input_num_width = estimated_num_width(est_num_inputs);
            let benchmark_name_width = estimated_str_width(benchmark_name);
            prefixed![(self) <-
                (" "),
                ("{: >input_num_width$}  ", "", input_num_width = input_num_width), // Account for the input number alignment
                ("{}{}{}",
                    ' '.repeat(benchmark_name_max_width - benchmark_name_width),
                    benchmark_name.style(self.format_options.sub_bench_name_style),
                    ':'.dimmed(),
                ),
            ];

            // And then the stats:
            self.print_stats::<M>(1, current_sum, current_max, current_min);

            // Now, update the state to indicate that we're waitin for the next
            // benchmark for this input:
            self.state = SuiteWaitingForNextBenchmarkForInput {
                suite_size,
                benchmark_name_max_width,
                est_num_inputs,
                remaining_benchmarks_for_input,
            };
        }

        // And finally, the case where we've finished all the benchmarks for an input
        // and need to move on to the next (potential) input:
        if let SuiteWaitingForNextBenchmarkForInput {
            remaining_benchmarks_for_input: 0,
            suite_size,
            benchmark_name_max_width,
            est_num_inputs,
        } = self.state
        {
            self.state = WaitingForNextInputInSuite {
                suite_size,
                benchmark_name_max_width,
                est_num_inputs,
            };
        }
    }

    fn ending_benchmark_suite(&mut self, _name: &'static str) {
        debug_assert!(matches!(
            self.state,
            State::WaitingForNextInputInSuite { .. }
        ));
        self.state = State::WaitingForNextTopLevel;

        prefixed![(self) <- ("\n")];
        writeln!(self.out, "\n\n").unwrap();
    }

    fn ended(&mut self) {
        debug_assert!(matches!(self.state, State::WaitingForNextTopLevel { .. }));
    }
}
