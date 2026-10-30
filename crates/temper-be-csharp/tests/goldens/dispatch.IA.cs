namespace Dispatch;

internal interface IA
{
    string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IA @this)
    {
        return "A wins!";
    }

    string B
    {
        get
        {
            return GetBDefault(this);
        }
    }

    protected static string GetBDefault(IA @this)
    {
        return "A's b";
    }
}
