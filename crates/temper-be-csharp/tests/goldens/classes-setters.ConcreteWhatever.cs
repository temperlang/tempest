namespace Setters;

public class ConcreteWhatever : IWhatever
{
    private int something;

    public ConcreteWhatever(int something)
    {
        this.something = something;
    }

    public int Something
    {
        get
        {
            return this.something;
        }
        set
        {
            this.something = value;
        }
    }
}
