using System;
using System.Collections;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

#if NET6_0_OR_GREATER
#nullable enable
#endif

namespace TemperLang.Core
{
    public class Async
    {
#if NET6_0_OR_GREATER
        public static void BreakPromise<T>(TaskCompletionSource<T> p, Exception? resolution = null)
#else
        public static void BreakPromise<T>(TaskCompletionSource<T> p, Exception resolution = null)
#endif
        {
            if (resolution == null)
            {
                resolution = new Exception();
            }
            try
            {
                p.SetException(resolution);
            }
            catch (InvalidOperationException)
            {
                // Temper semantics allow for multiple resolution where subsequent
                // resolutions are no-ops.
            }
        }

        public static void CompletePromise<T>(TaskCompletionSource<T> p, T resolution)
        {
            try
            {
                p.SetResult(resolution);
            }
            catch (InvalidOperationException)
            {
                // The note for breakPromise above applies here too.
            }
        }

        // Temper runs one async block at a time and switches only at `await`,
        // as the interpreter and JavaScript do (following Temper PR #532, which
        // does this for Python and Java).  Every step of every coroutine, its
        // launch and each resume after an awaited task settles, goes on one
        // FIFO queue that a single `temper-async` thread drains.  A task may
        // settle on any thread; that only queues the waiter, so
        // `CompletePromise` returns before the waiter runs.
        //
        // `Turn` is held while a step runs.  The generated `Main` holds it
        // while module initialization runs top-level code (`BeginTopLevel`),
        // so that no async block starts until that code is done.
        //
        // Generated code calls only the entry points (LaunchGeneratorAsync,
        // AwakeUpon, and BeginTopLevel and EndTopLevel), never the queue, so
        // the queue stays swappable: one queue per isolate or actor, many on
        // a thread pool, would make today's single queue the one-isolate case.
        private static readonly object Turn = new object();
        private static readonly Queue<Action> Ready = new Queue<Action>();
#if NET6_0_OR_GREATER
        private static Thread? runnerThread;
#else
        private static Thread runnerThread;
#endif

        private static void Schedule(Action step)
        {
            lock (Ready)
            {
                Ready.Enqueue(step);
                if (runnerThread == null)
                {
                    runnerThread = new Thread(RunSteps);
                    runnerThread.Name = "temper-async";
                    runnerThread.IsBackground = true;
                    runnerThread.Start();
                }
                Monitor.Pulse(Ready);
            }
        }

        private static void RunSteps()
        {
            while (true)
            {
                Action step;
                lock (Ready)
                {
                    while (Ready.Count == 0)
                    {
                        Monitor.Wait(Ready);
                    }
                    step = Ready.Dequeue();
                }
                lock (Turn)
                {
                    step();
                }
            }
        }

        /// <summary>
        /// Called from a main method before it runs module top-level code.
        /// Async blocks launched by that code wait for <see cref="EndTopLevel"/>.
        /// </summary>
        public static void BeginTopLevel()
        {
            Monitor.Enter(Turn);
        }

        /// <summary>Lets async blocks run after <see cref="BeginTopLevel"/>.</summary>
        public static void EndTopLevel()
        {
            Monitor.Exit(Turn);
        }

        /// <summary>Whether this thread is in a turn: a step, or top-level code.</summary>
        public static bool InTurn => Monitor.IsEntered(Turn);

#if NET6_0_OR_GREATER
        public static void LaunchGeneratorAsync(Func<IEnumerable<object?>> e)
#else
        public static void LaunchGeneratorAsync(Func<IEnumerable<object>> e)
#endif
        {
#if NET6_0_OR_GREATER
            IEnumerator<object?> enumerator = e().GetEnumerator();
#else
            IEnumerator<object> enumerator = e().GetEnumerator();
#endif
            IncrementUnresolved();
            Schedule(() => OneStep(enumerator));
        }

        private class AwaitResult
        {
            public readonly Task Task;
            public AwaitResult(Task task)
            {
                this.Task = task;
            }
        }

#if NET6_0_OR_GREATER
        public static System.Tuple<object?> AwakeUpon<T>(Task<T> task)
        {
            return new System.Tuple<object?>(new AwaitResult((Task)task));
        }
#else
        public static System.Tuple<object> AwakeUpon<T>(Task<T> task)
        {
            return new System.Tuple<object>(new AwaitResult((Task)task));
        }
#endif

#if NET6_0_OR_GREATER
        private static void OneStep(IEnumerator<object?> awaiter)
#else
        private static void OneStep(IEnumerator<object> awaiter)
#endif
        {
            bool continuing = false; // Is responsibility to decrement unresolved handed off?
            try
            {
                if (awaiter.MoveNext())
                {
#if NET6_0_OR_GREATER
                    object? yielded = awaiter.Current;
                    bool isOneTuple = yielded != null && yielded.GetType() == typeof(System.Tuple<object?>);
#else
                    object yielded = awaiter.Current;
                    bool isOneTuple = yielded != null && yielded.GetType() == typeof(System.Tuple<object>);
#endif
                    if (isOneTuple)
                    {
#if NET6_0_OR_GREATER
                        object? tupleContent = ((System.Tuple<object?>)yielded).Item1;
#else
                        object tupleContent = ((System.Tuple<object>)yielded).Item1;
#endif
                        if (tupleContent != null && tupleContent.GetType() == typeof(AwaitResult))
                        {
                            Task task = ((AwaitResult)tupleContent).Task;
                            // Runs on whichever thread settles the task, or
                            // at once if it has; either way, the resume
                            // waits its turn on the runner.
                            task.ContinueWith((_) =>
                            {
                                Schedule(() => OneStep(awaiter));
                            }, TaskContinuationOptions.ExecuteSynchronously);
                            continuing = true; // The continuation is responsible for decrementing
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine(ex.ToString());
                Console.WriteLine(new System.Diagnostics.StackTrace().ToString());
            }
            finally
            {
                if (!continuing)
                {
                    DecrementUnresolved();
                }
            }
        }

        // A count of pending promises
#if NET6_0_OR_GREATER
        private static volatile uint unresolvedCount = 0;
#else
        // Apparently no direct uint support on older dotnet.
        private static volatile int unresolvedCount = 0;
#endif
        private static void IncrementUnresolved()
        {
            allPromisesResolved.Reset();
            Interlocked.Increment(ref unresolvedCount);
        }
        private static void DecrementUnresolved()
        {
            // TODO Protect against subzero?
            long after = Interlocked.Decrement(ref unresolvedCount);
            if (after == 0)
            {
                allPromisesResolved.Set();
            }
        }

        // Signalled when the unresolved count goes to zero.
        private static ManualResetEvent allPromisesResolved = new ManualResetEvent(true);
        public static void WaitUntilSafeToExit()
        {
            if (InTurn)
            {
                throw new InvalidOperationException(
                    "WaitUntilSafeToExit inside an async turn would deadlock");
            }
            allPromisesResolved.WaitOne();
        }
    }
}
