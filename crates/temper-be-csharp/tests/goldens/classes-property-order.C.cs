namespace PropertyOrder;

internal class C
{
    private readonly int backed;

    public C(int? backed = null)
    {
        int backed2;
        if (backed == null)
        {
            PropertyOrderGlobal.console.Log("initializing backed");
            backed2 = 1;
        }
        else
        {
            backed2 = backed.Value;
        }
        this.backed = backed2;
    }

    public int Backed
    {
        get
        {
            return this.backed;
        }
    }

    public int Computed
    {
        get
        {
            PropertyOrderGlobal.console.Log("computing computed");
            return 1;
        }
    }
}
