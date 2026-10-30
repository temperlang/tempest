namespace CallOverrideFromSubtype;

internal interface IASplit
{
    string A()
    {
        return ADefault(this);
    }

    protected static string ADefault(IASplit @this)
    {
        return "A wins!";
    }
}
