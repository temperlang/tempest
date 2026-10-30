namespace InheritedGetter;

internal interface II
{
    string Place
    {
        get
        {
            return GetPlaceDefault(this);
        }
    }

    protected static string GetPlaceDefault(II @this)
    {
        return "Byron Bay";
    }
}
