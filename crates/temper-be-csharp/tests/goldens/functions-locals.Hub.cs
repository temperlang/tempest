using System;
using System.Collections.Generic;

namespace Locals;

internal class Hub<T>
{
    private readonly IList<Action<T>> handlers;

    public Hub()
    {
        this.handlers = new List<Action<T>>();
    }

    public void OnAction(Action<T> handler)
    {
        this.handlers.Add(handler);
    }

    public void ActionPerformed(T item)
    {
        {
            int i = 0;
            while (i < this.handlers.Count)
            {
                this.handlers[i](item);
                i = i + 1;
            }
        }
    }
}
