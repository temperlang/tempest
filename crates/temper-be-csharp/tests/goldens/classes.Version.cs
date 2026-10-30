namespace Classes;

internal class Version
{
    private readonly int major;
    private readonly int minor;

    public Version(int major, int minor)
    {
        this.major = major;
        this.minor = minor;
    }

    public int Major
    {
        get
        {
            return this.major;
        }
    }

    public int Minor
    {
        get
        {
            return this.minor;
        }
    }

    public int CompareTo(Version other)
    {
        if (this.major != other.major)
        {
            return this.major.CompareTo(other.major);
        }
        else
        {
            return this.minor.CompareTo(other.minor);
        }
    }
}
