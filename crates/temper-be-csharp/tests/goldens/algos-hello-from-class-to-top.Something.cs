namespace HelloFromClassToTop;

internal class Something
{
    private readonly string message;

    public Something(string message)
    {
        this.message = message;
    }

    public void SayMessage()
    {
        HelloFromClassToTopGlobal.Say(this.message);
    }
}
