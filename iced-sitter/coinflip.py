import random

class CoinFlip:
    def coinflip():
        coin = random.randint(0,1)
        if coin == 0:
            return "heads"
        else:
            return "tails"
    print(coinflip())