namespace ContentTags;

public class EventParseEffect : IEffect
{
    private readonly Mark @event;

    public EventParseEffect(Mark @event)
    {
        this.@event = @event;
    }

    public Mark Event
    {
        get
        {
            return this.@event;
        }
    }

    public void EnactOn(Buffer c)
    {
        c.Mark(this.@event);
    }
}
