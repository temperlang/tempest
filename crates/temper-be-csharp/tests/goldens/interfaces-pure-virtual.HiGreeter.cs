namespace PureVirtual;

internal class HiGreeter : IGreeter
{
    public HiGreeter()
    {
    }

    public void Greet()
    {
        PureVirtualGlobal.console.Log("Hi, virtually!");
    }
}
