namespace Empty;

internal class HiGreeter : IGreeter
{
    public HiGreeter()
    {
    }

    public void Greet()
    {
        EmptyGlobal.console.Log("Hi!");
    }
}
