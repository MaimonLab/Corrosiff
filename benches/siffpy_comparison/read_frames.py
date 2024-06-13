"""
Directly calls siffio rather than
using the `SiffReader` methods for
a fairer comparison to the `Rust`
code (which doesn't have to deal with
Python either).
"""
import siffpy

from local_consts import small_path, large_path


def test_read_small(sr):
    sr.siffio.get_frames(frames = list(range(40)))

def test_read_large(sr):
    sr.siffio.get_frames(frames = list(range(50000)))

if __name__ == '__main__':
    import timeit
    sr_large = siffpy.SiffReader(large_path)
    print("Opened large")
    print(
        "Get 50000 large frames:\n",
        timeit.timeit(
            "test(sr)",
            setup="from __main__ import test_read_large as test"
            + "\nfrom __main__ import sr_large as sr",
            number = 30,
        )/30 , "sec per iter"
    )
    sr_small = siffpy.SiffReader(small_path)
    print("Opened small")
    print(
        "Get 40 small frames:\n",
        timeit.timeit(
            "test(sr)",
            setup="from __main__ import test_read_small as test"
            + "\nfrom __main__ import sr_small as sr",
            number = 100,
        )/100 * 1000 , "msec per iter"
    )