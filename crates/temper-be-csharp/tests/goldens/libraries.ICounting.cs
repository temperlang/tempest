using System.Collections.Generic;
using TemperLang.Core;

namespace Libraries;

public interface ICounting<T> : IStepper<T>
{
    int Count { get; set; }

    T Made();

    new IReadOnlyList<T> Step()
    {
        return StepDefault(this);
    }

    protected static IReadOnlyList<T> StepDefault(ICounting<T> @this)
    {
        @this.Count = @this.Count + 1;
        return Listed.CreateReadOnlyList<T>(@this.Made());
    }
}
