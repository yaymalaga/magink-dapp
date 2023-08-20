import { Values } from "../types";
import { FormikHelpers } from "formik";

import { useMaginkContract } from "./useMaginkContract";

export const useSubmitHandler = () => {
  const { claim, mint } = useMaginkContract();
  
  return async (
    values: Values,
    { setSubmitting }: FormikHelpers<Values>,
    badges: number
  ) => {
    let action = badges < 9 ? claim : mint;
    
    if (badges < 9) {
      console.log("send claim Tx");
    } else {
      console.log("send mint Tx");
    }

    const args = undefined;
    const options = undefined;

    action?.signAndSend(args, options, (result, _api, error) => {
      if (error) {
        console.error(JSON.stringify(error));
        setSubmitting(false);
      }

      if (!result?.status.isInBlock) return;

      setSubmitting(false);
    });
  };
};
