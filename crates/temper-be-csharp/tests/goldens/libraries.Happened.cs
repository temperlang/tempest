namespace Libraries;

public class Happened<T> : IEffect<T>
{
    private readonly T @event;

    public Happened(T @event)
    {
        this.@event = @event;
    }

    public T Event
    {
        get
        {
            return this.@event;
        }
    }
}
