namespace PropertyMembers;

internal class C : IJ
{
    private string x;

    public C(string x)
    {
        this.x = x;
    }

    public string X
    {
        get
        {
            return this.x;
        }
        set
        {
            this.x = value;
        }
    }
}
