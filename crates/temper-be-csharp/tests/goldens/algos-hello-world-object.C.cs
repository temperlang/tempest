namespace HelloWorldObject;

internal class C
{
    private readonly string x;
    private readonly string y;

    public C(string x, string y)
    {
        this.x = x;
        this.y = y;
    }

    public string X
    {
        get
        {
            return this.x;
        }
    }

    public string Y
    {
        get
        {
            return this.y;
        }
    }

    public void Echo(string input)
    {
        HelloWorldObjectGlobal.console.Log(input);
    }
}
