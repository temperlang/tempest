namespace Dispatch;

internal interface IC : IA
{
    new string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IC @this)
    {
        return "C wins!";
    }

    new string B
    {
        get
        {
            return GetBDefault(this);
        }
    }

    protected static string GetBDefault(IC @this)
    {
        return "C's b";
    }
}
