using System.Collections.Generic;

namespace Libraries;

public interface IStepper<T>
{
    IReadOnlyList<T> Step();
}
