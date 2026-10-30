namespace CallOverrideFromSubtype;

internal class Slippy : ISlappy
{
    public Slippy()
    {
    }

    public bool CheckOverridden()
    {
        return this.Thingums == 1;
    }

    public bool CheckOwn()
    {
        return this.Thingumses == 2;
    }

    public int Thingums
    {
        get
        {
            return 3;
        }
    }

    public int Thingumses
    {
        get
        {
            return 4;
        }
    }
}
