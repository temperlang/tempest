namespace InheritedGetter;

internal class C : II
{
    public C()
    {
    }

    public string Place
    {
        get
        {
            return II.GetPlaceDefault(this);
        }
    }
}
