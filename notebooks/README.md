# Notebooks
In this directory we provide a set of notebooks where the experiments of the paper were conducted. The contents of each notebook are:
- **1_Data_Fetching.ipynb**. In this notebook we execute ClassRank to select the 100 most important classes from Wikidata. We then fetch the ids of the instances from these classes. This data is used to then execute the edit history fetching scripts.
- **2_Data_Exploration.ipynb**. In this notebook we perform the analysis of edit history information indexed in the MongoDB database.
- **3_KG_Refinement_Systems.ipynb**. In this notebook we create the RDF dataset from the JSON data and train our proposed type prediction methods with custom negative sampling techniques,
- **4_Model_Evaluation.ipynb**. In this notebook we perform the evaluation of the models trained previously.

Each notebook usually depends on outputs from the previous one. The outputs of each notebook are stored in the *output* folder. Due to size limitations most of these outputs are not available in the repository, so we recommend downloading them from the [following link](https://unioviedo-my.sharepoint.com/:f:/g/personal/uo251513_uniovi_es/EuN7_OxvEM5Lob5858cJzn4BlSnhTlvj5f9JkVi11d90Hg?e=zA3ZCu) and replacing this *output* folder with the one that was downloaded. 