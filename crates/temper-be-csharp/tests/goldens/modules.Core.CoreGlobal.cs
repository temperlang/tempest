namespace Modules.Core;

public static class CoreGlobal
{
    public static int Share(int n, int d)
    {
        try
        {
            return global::TemperLang.Core.Core.Div(n, d);
        }
        catch
        {
            return 0;
        }
    }
}
