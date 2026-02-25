import pandas as pd
import matplotlib.pyplot as plt

df = pd.read_csv("data.csv")
print(df.describe())

df.plot(kind="bar", x="category", y="value")
plt.savefig("output.png")
