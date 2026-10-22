import copy
import unittest
from summarize_benchmark import validate_complete


class CompletenessTest(unittest.TestCase):
    def test_missing_duplicate_and_complete_campaign(self):
        rows = [{"orders":str(size),"query":query,"side":side,"repeat":str(repeat),"warmup":str(repeat == 0).lower(),"rows":"2"}
                for size in [1000,100000,1000000]
                for query in ["skewed_join","aggregate","selective_join","scan","empty"]
                for side in ["optd","datafusion"]
                for repeat in range(6)]
        validate_complete(rows)
        with self.assertRaises(ValueError):
            validate_complete(rows[:-1])
        duplicate = copy.deepcopy(rows)
        duplicate[-1]["repeat"] = "1"
        with self.assertRaises(ValueError):
            validate_complete(duplicate)
        with self.assertRaises(ValueError):
            validate_complete(rows[:60])


if __name__ == "__main__":
    unittest.main()
