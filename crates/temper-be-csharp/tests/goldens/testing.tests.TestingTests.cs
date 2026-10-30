using Microsoft.VisualStudio.TestTools.UnitTesting;
using TemperLang.Std.Testing;

namespace Testing;

[TestClass]
public class TestingTests
{
    [TestMethod]
    public void TwiceWorks()
    {
        Test test = new Test();
        try
        {
            int x = TestingGlobal.Twice(2);
            TestingGlobal.runs = TestingGlobal.runs + x + TestingGlobal.@base;
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void NamesPunctuationQuotesAndÜnïcode()
    {
        Test t = new Test();
        try
        {
            TestingGlobal.Twice(TestingGlobal.runs);
        }
        finally
        {
            t.SoftFailToHard();
        }
    }
}
